//! USB device: serial log stream and picotool-compatible reset interface.
//!
//! One USB-C cable gives the board:
//!
//! 1. A **CDC-ACM serial port** carrying the `defmt` log stream (see
//!    [`crate::logger`]); decode it with `defmt-print` (`scripts/fw-logs`).
//! 2. A **reset interface** (vendor class 0xFF/0x00/0x01, the same one the
//!    Pico SDK exposes). `picotool load -f` / `picotool reboot -f -u` use it to
//!    put the board into the ROM bootloader, so flashing needs no BOOTSEL button.

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::info;
use embassy_futures::join::join3;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::control::{OutResponse, Recipient, Request, RequestType};
use embassy_usb::{Builder, Config, Handler};
use static_cell::StaticCell;

/// Raspberry Pi's vendor id and the Pico SDK's CDC product id. `picotool` only
/// looks at devices with this vendor id. Ask Raspberry Pi for a product id of
/// your own before shipping units to other people.
const USB_VID: u16 = 0x2E8A;
const USB_PID: u16 = 0x000A;

// Pico SDK reset interface (see `pico/stdio_usb/reset_interface.h`).
const RESET_CLASS: u8 = 0xFF;
const RESET_SUBCLASS: u8 = 0x00;
const RESET_PROTOCOL: u8 = 0x01;
const RESET_REQUEST_BOOTSEL: u8 = 0x01;
const RESET_REQUEST_FLASH: u8 = 0x02;

#[derive(Clone, Copy)]
enum ResetKind {
    /// Reboot into the ROM USB bootloader (BOOTSEL mode).
    Bootloader,
    /// Plain reboot back into the application.
    Application,
}

static RESET: Signal<CriticalSectionRawMutex, ResetKind> = Signal::new();

/// True while the USB host has the serial port configured.
pub static HOST_CONNECTED: AtomicBool = AtomicBool::new(false);

struct ResetHandler {
    interface: u8,
}

impl Handler for ResetHandler {
    fn control_out(&mut self, req: Request, _data: &[u8]) -> Option<OutResponse> {
        if req.request_type != RequestType::Class
            || req.recipient != Recipient::Interface
            || req.index != self.interface as u16
        {
            return None;
        }
        match req.request {
            RESET_REQUEST_BOOTSEL => RESET.signal(ResetKind::Bootloader),
            RESET_REQUEST_FLASH => RESET.signal(ResetKind::Application),
            _ => return Some(OutResponse::Rejected),
        }
        Some(OutResponse::Accepted)
    }

    fn configured(&mut self, configured: bool) {
        HOST_CONNECTED.store(configured, Ordering::Relaxed);
    }
}

/// Runs the USB device forever. `serial` is shown as the USB serial number so
/// several boards on one host stay distinguishable.
pub async fn run(driver: Driver<'static, USB>, serial: &'static str) -> ! {
    static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 64]> = StaticCell::new();
    static MSOS_DESC: StaticCell<[u8; 64]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static CDC_STATE: StaticCell<State> = StaticCell::new();
    static RESET_HANDLER: StaticCell<ResetHandler> = StaticCell::new();

    let mut config = Config::new(USB_VID, USB_PID);
    config.manufacturer = Some("RA Lighting");
    config.product = Some("hydra nano");
    config.serial_number = Some(serial);
    config.max_power = 100;

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESC.init([0; 256]),
        BOS_DESC.init([0; 64]),
        MSOS_DESC.init([0; 64]),
        CONTROL_BUF.init([0; 64]),
    );

    let mut cdc = CdcAcmClass::new(&mut builder, CDC_STATE.init(State::new()), 64);

    // Reset interface: a bare vendor interface with one alt setting.
    let mut function = builder.function(RESET_CLASS, RESET_SUBCLASS, RESET_PROTOCOL);
    let mut interface = function.interface();
    let interface_number = interface.interface_number().0;
    let _alt = interface.alt_setting(RESET_CLASS, RESET_SUBCLASS, RESET_PROTOCOL, None);
    drop(function);
    builder.handler(RESET_HANDLER.init(ResetHandler {
        interface: interface_number,
    }));

    let mut device = builder.build();
    info!("USB: serial log + reset interface ready");

    let usb_fut = device.run();

    // Streams buffered log bytes to the host whenever a port is open.
    let log_fut = async {
        loop {
            cdc.wait_connection().await;
            // Every connection starts with a replay of the boot log.
            let mut pos = 0usize;
            let mut packet = [0u8; 64];
            loop {
                if !cdc.dtr() {
                    // Port not open (wait_connection only means "enumerated",
                    // so this must yield): replay the boot log on the next open.
                    pos = 0;
                    Timer::after_millis(50).await;
                    continue;
                }
                let n = crate::logger::read(&mut pos, &mut packet);
                if n == 0 {
                    Timer::after_millis(20).await;
                    continue;
                }
                if cdc.write_packet(&packet[..n]).await.is_err() {
                    break; // host went away: wait for the next connection
                }
            }
        }
    };

    // Performs a requested reset after the control transfer has completed.
    let reset_fut = async {
        let kind = RESET.wait().await;
        Timer::after_millis(50).await;
        match kind {
            ResetKind::Bootloader => embassy_rp::rom_data::reset_to_usb_boot(0, 0),
            ResetKind::Application => cortex_m::peripheral::SCB::sys_reset(),
        }
    };

    join3(usb_fut, log_fut, reset_fut).await;
    unreachable!()
}
