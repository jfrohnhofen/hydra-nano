//! `defmt` global logger that fans every frame out to two sinks:
//!
//! - **RTT** (for a debug probe, optional), and
//! - a **ring buffer drained over USB serial** by [`crate::usb`].
//!
//! `defmt` allows exactly one global logger, which is why this replaces
//! `defmt-rtt` instead of sitting next to it. Both sinks never block: if RTT or
//! the ring is full, frames are dropped, so logging can never stall the
//! firmware (for example while no USB host is listening).
//!
//! The first [`HISTORY_SIZE`] bytes of log frames (the boot log) go to a history
//! buffer that is never consumed: every new USB connection replays it from the
//! start, so the boot log is there no matter when (or how often) a host opens
//! the port. Frames after that stream live through a ring that keeps the oldest
//! frames when full. Frames are all-or-nothing: a frame that does not fit is
//! removed again, never left half-written.

use core::ptr::addr_of_mut;
use core::sync::atomic::{AtomicBool, Ordering};

use heapless::{Deque, Vec};
use rtt_target::UpChannel;

/// Bytes of live log buffered for the USB serial port (after the history).
const USB_RING_SIZE: usize = 4096;
/// Bytes of boot log replayed to every new USB connection.
const HISTORY_SIZE: usize = 3072;

static TAKEN: AtomicBool = AtomicBool::new(false);
static mut CS_RESTORE: critical_section::RestoreState = critical_section::RestoreState::invalid();
static mut ENCODER: defmt::Encoder = defmt::Encoder::new();
static mut RTT: Option<UpChannel> = None;
static mut USB_RING: Deque<u8, USB_RING_SIZE> = Deque::new();
static mut HISTORY: Vec<u8, HISTORY_SIZE> = Vec::new();
/// Set once a frame no longer fit the history; later frames go to the ring.
static mut HISTORY_FULL: bool = false;
/// Whether the current frame is being written to the history (else the ring).
static mut FRAME_IN_HISTORY: bool = false;
/// Sink length when the current frame started, to roll back an overflowing frame.
static mut FRAME_START: usize = 0;
static mut FRAME_OVERFLOW: bool = false;

defmt::timestamp!("{=u64:us}", embassy_time::Instant::now().as_micros());

/// Sets up the RTT channel. Call once, first thing in `main`. Logging before
/// this still reaches the USB ring; it is only missing from RTT.
pub fn init() {
    let channels = rtt_target::rtt_init! {
        up: {
            0: {
                size: 1024,
                name: "defmt"
            }
        }
    };
    critical_section::with(|_| {
        // SAFETY: inside a critical section, the logger cannot run concurrently.
        unsafe { RTT = Some(channels.up.0) };
    });
}

/// Copies the next chunk of log bytes into `out`; returns how many.
///
/// `pos` is the reader's position in the replayable history: start every new
/// connection at 0. Once the history is exhausted, bytes are taken from the
/// live ring instead.
pub fn read(pos: &mut usize, out: &mut [u8]) -> usize {
    critical_section::with(|_| {
        // SAFETY: inside a critical section, the logger cannot run concurrently.
        let (history, full, ring) = unsafe {
            (
                &*addr_of_mut!(HISTORY),
                HISTORY_FULL,
                &mut *addr_of_mut!(USB_RING),
            )
        };
        if *pos < history.len() {
            let n = (history.len() - *pos).min(out.len());
            out[..n].copy_from_slice(&history[*pos..*pos + n]);
            *pos += n;
            return n;
        }
        if !full {
            return 0; // history still growing: wait for more
        }
        let mut n = 0;
        while n < out.len() {
            match ring.pop_front() {
                Some(b) => {
                    out[n] = b;
                    n += 1;
                }
                None => break,
            }
        }
        n
    })
}

#[defmt::global_logger]
struct Logger;

unsafe impl defmt::Logger for Logger {
    fn acquire() {
        // SAFETY: paired with the `release` in `Logger::release`.
        let restore = unsafe { critical_section::acquire() };

        if TAKEN.load(Ordering::Relaxed) {
            panic!("defmt logger taken reentrantly")
        }
        TAKEN.store(true, Ordering::Relaxed);

        // SAFETY: the critical section is held, so access is exclusive.
        unsafe {
            CS_RESTORE = restore;
            FRAME_IN_HISTORY = !HISTORY_FULL;
            FRAME_START = if FRAME_IN_HISTORY {
                (&*addr_of_mut!(HISTORY)).len()
            } else {
                (&*addr_of_mut!(USB_RING)).len()
            };
            FRAME_OVERFLOW = false;
            (*addr_of_mut!(ENCODER)).start_frame(do_write);
        }
    }

    unsafe fn flush() {}

    unsafe fn release() {
        (*addr_of_mut!(ENCODER)).end_frame(do_write);

        // Drop a frame that did not fully fit instead of leaving half of it.
        if FRAME_OVERFLOW {
            if FRAME_IN_HISTORY {
                (*addr_of_mut!(HISTORY)).truncate(FRAME_START);
                // History is complete; everything after goes to the ring.
                HISTORY_FULL = true;
            } else {
                let ring = &mut *addr_of_mut!(USB_RING);
                while ring.len() > FRAME_START {
                    ring.pop_back();
                }
            }
        }

        TAKEN.store(false, Ordering::Relaxed);
        let restore = CS_RESTORE;
        critical_section::release(restore);
    }

    unsafe fn write(bytes: &[u8]) {
        (*addr_of_mut!(ENCODER)).write(bytes, do_write);
    }
}

fn do_write(bytes: &[u8]) {
    // SAFETY: only called from the logger, which holds the critical section.
    unsafe {
        if let Some(channel) = (*addr_of_mut!(RTT)).as_mut() {
            channel.write(bytes);
        }

        if FRAME_OVERFLOW {
            return;
        }
        if FRAME_IN_HISTORY {
            let history = &mut *addr_of_mut!(HISTORY);
            if history.extend_from_slice(bytes).is_err() {
                FRAME_OVERFLOW = true;
            }
            return;
        }
        let ring = &mut *addr_of_mut!(USB_RING);
        if ring.capacity() - ring.len() < bytes.len() {
            FRAME_OVERFLOW = true;
            return;
        }
        for &b in bytes {
            let _ = ring.push_back(b);
        }
    }
}
