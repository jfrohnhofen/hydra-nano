//! Persistent configuration schema and binary serialization for hydra-nano.
//!
//! Stores:
//! - Global PWM frequency
//! - 8 channel settings (mode, DMX mapping, loss config)
//! - Network settings (DHCP vs static IP, prefix, gateway)
//! - Hostname (.local)
//!
//! Stored in persistent flash storage as a 512-byte block with a 16-byte
//! header:
//! - [0..4]   "HYDR" (magic)
//! - [4..6]   version (u16 LE, currently 1)
//! - [6..8]   payload_len (u16 LE)
//! - [8..12]  crc32 (u32 LE of payload bytes [16..16+payload_len])
//! - [12..16] reserved
//! - [16..16+len] payload
//! - [16+len..512] padding (0xFF)

use crate::channel::{ChannelMode, DmxSlot, GroupClass};
use crate::loss::{LossBehavior, LossConfig};

pub const GROUP_COUNT: usize = 4;
pub const CHANNELS_PER_GROUP: usize = 2;
pub const CHANNEL_COUNT: usize = 8;

pub const CONFIG_MAGIC: [u8; 4] = *b"HYDR";
pub const CONFIG_VERSION: u16 = 1;
pub const CONFIG_BLOCK_SIZE: usize = 512;

pub const DEFAULT_LOSS_TIMEOUT_MS: u32 = 2500;
pub const MIN_LOSS_TIMEOUT_MS: u32 = 100;
pub const MAX_LOSS_TIMEOUT_MS: u32 = 60_000;
pub const DEFAULT_FADE_MS: u32 = 1000;

/// Standard IEEE 802.3 CRC32 (Ethernet polynomial 0xEDB88320).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSetting {
    pub mode: ChannelMode,
    pub loss: LossConfig,
}

impl ChannelSetting {
    pub const fn const_default() -> Self {
        Self {
            mode: ChannelMode::Off,
            loss: LossConfig {
                timeout_ms: DEFAULT_LOSS_TIMEOUT_MS,
                behavior: LossBehavior::Blackout,
            },
        }
    }
}

impl Default for ChannelSetting {
    fn default() -> Self {
        Self::const_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupSetting {
    pub class: GroupClass,
}

impl Default for GroupSetting {
    fn default() -> Self {
        Self {
            class: GroupClass::Pixel,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustomHostname {
    pub bytes: [u8; 32],
    pub len: u8,
}

impl CustomHostname {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; 32],
            len: 0,
        }
    }

    /// Build a valid DNS-label hostname from arbitrary user input.
    ///
    /// Keeps ASCII letters (lower-cased), digits and `-`; other characters are
    /// dropped, leading/trailing `-` are stripped and the result is limited to
    /// 32 bytes. An empty result yields the default hostname.
    pub fn from_str(s: &str) -> Self {
        let mut bytes = [0u8; 32];
        let mut len = 0usize;
        for c in s.trim().chars() {
            if len >= 32 {
                break;
            }
            let c = c.to_ascii_lowercase();
            if c.is_ascii_alphanumeric() || (c == '-' && len > 0) {
                bytes[len] = c as u8;
                len += 1;
            }
        }
        while len > 0 && bytes[len - 1] == b'-' {
            len -= 1;
            bytes[len] = 0;
        }
        Self {
            bytes,
            len: len as u8,
        }
    }

    pub fn as_str(&self) -> &str {
        if self.len == 0 {
            "hydra"
        } else {
            core::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("hydra")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetConfig {
    pub dhcp: bool,
    pub static_ip: [u8; 4],
    pub static_prefix: u8,
    pub static_gateway: Option<[u8; 4]>,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            dhcp: true,
            static_ip: [169, 254, 246, 176],
            static_prefix: 16,
            static_gateway: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppConfig {
    pub pwm_freq_hz: u32,
    pub channels: [ChannelSetting; CHANNEL_COUNT],
    pub net: NetConfig,
    pub hostname: CustomHostname,
}

impl AppConfig {
    pub const fn const_default() -> Self {
        Self {
            pwm_freq_hz: crate::channel::DEFAULT_PWM_FREQ_HZ,
            channels: [ChannelSetting::const_default(); CHANNEL_COUNT],
            net: NetConfig {
                dhcp: true,
                static_ip: [169, 254, 246, 176],
                static_prefix: 16,
                static_gateway: None,
            },
            hostname: CustomHostname::empty(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::const_default()
    }
}

/// Checks a static IPv4 setup. Returns a user-facing reason on failure.
pub fn validate_static_net(
    ip: [u8; 4],
    prefix: u8,
    gateway: Option<[u8; 4]>,
) -> Result<(), &'static str> {
    if !(1..=32).contains(&prefix) {
        return Err("Subnet prefix must be between 1 and 32.");
    }
    if ip == [0, 0, 0, 0] || ip[0] == 127 || ip[0] >= 224 {
        return Err("IP address must be a usable unicast address.");
    }
    let mask = if prefix == 32 {
        u32::MAX
    } else {
        !(u32::MAX >> prefix)
    };
    let ip_n = u32::from_be_bytes(ip);
    if prefix < 31 && (ip_n & !mask == 0 || ip_n & !mask == !mask) {
        return Err("IP address is the network or broadcast address of its subnet.");
    }
    if let Some(gw) = gateway {
        let gw_n = u32::from_be_bytes(gw);
        if gw_n & mask != ip_n & mask {
            return Err("Gateway is not inside the IP address's subnet.");
        }
        if gw == ip {
            return Err("Gateway must differ from the IP address.");
        }
    }
    Ok(())
}

/// Forces stored channel parameters into the ranges the UI and renderer accept.
fn sanitize_mode(mode: ChannelMode) -> ChannelMode {
    let uni = |u: u16| u.clamp(1, 63999);
    match mode {
        ChannelMode::Off => ChannelMode::Off,
        ChannelMode::Ws2812 {
            universe,
            start_channel,
            pixel_count,
        } => ChannelMode::Ws2812 {
            universe: uni(universe),
            start_channel: start_channel.clamp(1, 512),
            pixel_count: pixel_count.clamp(1, 1000),
        },
        ChannelMode::Ws2812Fill {
            universe,
            start_channel,
            pixel_count,
        } => ChannelMode::Ws2812Fill {
            universe: uni(universe),
            start_channel: start_channel.clamp(1, 510),
            pixel_count: pixel_count.clamp(1, 1000),
        },
        ChannelMode::Fire {
            pixel_count,
            control,
        } => ChannelMode::Fire {
            pixel_count: pixel_count.clamp(1, 1000),
            control: DmxSlot {
                universe: uni(control.universe),
                channel: control.channel.clamp(1, 510),
            },
        },
        ChannelMode::Pwm {
            universe,
            dmx_channel,
        } => ChannelMode::Pwm {
            universe: uni(universe),
            dmx_channel: dmx_channel.clamp(1, 512),
        },
    }
}

impl AppConfig {
    /// The signal-loss timeout used to judge whether a stream is "live":
    /// the longest timeout among configured outputs.
    pub fn loss_timeout_ms(&self) -> u32 {
        self.channels
            .iter()
            .filter(|c| !matches!(c.mode, ChannelMode::Off))
            .map(|c| c.loss.timeout_ms)
            .max()
            .unwrap_or(DEFAULT_LOSS_TIMEOUT_MS)
    }

    /// Encode this configuration into a 512-byte flash block with header and CRC32.
    pub fn encode(&self, out: &mut [u8; CONFIG_BLOCK_SIZE]) {
        out.fill(0xFF);
        out[0..4].copy_from_slice(&CONFIG_MAGIC);
        out[4..6].copy_from_slice(&CONFIG_VERSION.to_le_bytes());

        let mut idx = 16;

        // 1. Global PWM Frequency
        out[idx..idx + 4].copy_from_slice(&self.pwm_freq_hz.to_le_bytes());
        idx += 4;

        // 2. Channels (8 channels)
        for ch in &self.channels {
            match ch.mode {
                ChannelMode::Off => {
                    out[idx] = 0;
                    out[idx + 1..idx + 9].fill(0);
                }
                ChannelMode::Ws2812 {
                    universe,
                    start_channel,
                    pixel_count,
                } => {
                    out[idx] = 1;
                    out[idx + 1..idx + 3].copy_from_slice(&universe.to_le_bytes());
                    out[idx + 3..idx + 5].copy_from_slice(&start_channel.to_le_bytes());
                    out[idx + 5..idx + 7].copy_from_slice(&pixel_count.to_le_bytes());
                    out[idx + 7..idx + 9].fill(0);
                }
                ChannelMode::Ws2812Fill {
                    universe,
                    start_channel,
                    pixel_count,
                } => {
                    out[idx] = 2;
                    out[idx + 1..idx + 3].copy_from_slice(&universe.to_le_bytes());
                    out[idx + 3..idx + 5].copy_from_slice(&start_channel.to_le_bytes());
                    out[idx + 5..idx + 7].copy_from_slice(&pixel_count.to_le_bytes());
                    out[idx + 7..idx + 9].fill(0);
                }
                ChannelMode::Fire {
                    pixel_count,
                    control,
                } => {
                    out[idx] = 3;
                    out[idx + 1..idx + 3].copy_from_slice(&pixel_count.to_le_bytes());
                    out[idx + 3..idx + 5].copy_from_slice(&control.universe.to_le_bytes());
                    out[idx + 5..idx + 7].copy_from_slice(&control.channel.to_le_bytes());
                    out[idx + 7..idx + 9].fill(0);
                }
                ChannelMode::Pwm {
                    universe,
                    dmx_channel,
                } => {
                    out[idx] = 4;
                    out[idx + 1..idx + 3].copy_from_slice(&universe.to_le_bytes());
                    out[idx + 3..idx + 5].copy_from_slice(&dmx_channel.to_le_bytes());
                    out[idx + 5..idx + 9].fill(0);
                }
            }
            idx += 9;

            match ch.loss.behavior {
                LossBehavior::Blackout => {
                    out[idx] = 0;
                    out[idx + 1..idx + 5].fill(0);
                }
                LossBehavior::Hold => {
                    out[idx] = 1;
                    out[idx + 1..idx + 5].fill(0);
                }
                LossBehavior::Fade { ms } => {
                    out[idx] = 2;
                    out[idx + 1..idx + 5].copy_from_slice(&ms.to_le_bytes());
                }
            }
            idx += 5;

            out[idx..idx + 4].copy_from_slice(&ch.loss.timeout_ms.to_le_bytes());
            idx += 4;
        }

        // 3. Network config
        out[idx] = if self.net.dhcp { 1 } else { 0 };
        idx += 1;
        out[idx..idx + 4].copy_from_slice(&self.net.static_ip);
        idx += 4;
        out[idx] = self.net.static_prefix;
        idx += 1;
        if let Some(gw) = self.net.static_gateway {
            out[idx] = 1;
            out[idx + 1..idx + 5].copy_from_slice(&gw);
        } else {
            out[idx] = 0;
            out[idx + 1..idx + 5].fill(0);
        }
        idx += 5;

        // 4. Hostname
        out[idx] = self.hostname.len;
        idx += 1;
        out[idx..idx + 32].copy_from_slice(&self.hostname.bytes);
        idx += 32;

        let payload_len = (idx - 16) as u16;
        out[6..8].copy_from_slice(&payload_len.to_le_bytes());
        let crc = crc32(&out[16..idx]);
        out[8..12].copy_from_slice(&crc.to_le_bytes());
        out[12..16].fill(0);
    }

    /// Decode a 512-byte flash block. Returns `None` if magic, version, or CRC32 fails.
    pub fn decode(block: &[u8; CONFIG_BLOCK_SIZE]) -> Option<Self> {
        if block[0..4] != CONFIG_MAGIC {
            return None;
        }
        let version = u16::from_le_bytes([block[4], block[5]]);
        if version != CONFIG_VERSION {
            return None;
        }
        let payload_len = u16::from_le_bytes([block[6], block[7]]) as usize;
        if payload_len == 0 || payload_len > (CONFIG_BLOCK_SIZE - 16) {
            return None;
        }
        let stored_crc = u32::from_le_bytes([block[8], block[9], block[10], block[11]]);
        let calc_crc = crc32(&block[16..16 + payload_len]);
        if stored_crc != calc_crc {
            return None;
        }

        let mut idx = 16;
        let freq = u32::from_le_bytes(block[idx..idx + 4].try_into().ok()?);
        idx += 4;
        let pwm_freq_hz = if (crate::channel::MIN_PWM_FREQ_HZ..=crate::channel::MAX_PWM_FREQ_HZ)
            .contains(&freq)
        {
            freq
        } else {
            crate::channel::DEFAULT_PWM_FREQ_HZ
        };

        let mut channels = [ChannelSetting::default(); CHANNEL_COUNT];
        for i in 0..CHANNEL_COUNT {
            let mode_tag = block[idx];
            let mode = match mode_tag {
                1 => {
                    let u = u16::from_le_bytes(block[idx + 1..idx + 3].try_into().ok()?);
                    let d = u16::from_le_bytes(block[idx + 3..idx + 5].try_into().ok()?);
                    let p = u16::from_le_bytes(block[idx + 5..idx + 7].try_into().ok()?);
                    ChannelMode::Ws2812 {
                        universe: u,
                        start_channel: d,
                        pixel_count: p,
                    }
                }
                2 => {
                    let u = u16::from_le_bytes(block[idx + 1..idx + 3].try_into().ok()?);
                    let d = u16::from_le_bytes(block[idx + 3..idx + 5].try_into().ok()?);
                    let p = u16::from_le_bytes(block[idx + 5..idx + 7].try_into().ok()?);
                    ChannelMode::Ws2812Fill {
                        universe: u,
                        start_channel: d,
                        pixel_count: p,
                    }
                }
                3 => {
                    let p = u16::from_le_bytes(block[idx + 1..idx + 3].try_into().ok()?);
                    let cu = u16::from_le_bytes(block[idx + 3..idx + 5].try_into().ok()?);
                    let cc = u16::from_le_bytes(block[idx + 5..idx + 7].try_into().ok()?);
                    ChannelMode::Fire {
                        pixel_count: p,
                        control: DmxSlot {
                            universe: cu,
                            channel: cc,
                        },
                    }
                }
                4 => {
                    let u = u16::from_le_bytes(block[idx + 1..idx + 3].try_into().ok()?);
                    let d = u16::from_le_bytes(block[idx + 3..idx + 5].try_into().ok()?);
                    ChannelMode::Pwm {
                        universe: u,
                        dmx_channel: d,
                    }
                }
                _ => ChannelMode::Off,
            };
            idx += 9;

            let behavior = match block[idx] {
                1 => LossBehavior::Hold,
                2 => {
                    let ms = u32::from_le_bytes(block[idx + 1..idx + 5].try_into().ok()?);
                    LossBehavior::Fade { ms }
                }
                _ => LossBehavior::Blackout,
            };
            idx += 5;

            let timeout_ms = u32::from_le_bytes(block[idx..idx + 4].try_into().ok()?);
            idx += 4;

            channels[i] = ChannelSetting {
                mode: sanitize_mode(mode),
                loss: LossConfig {
                    timeout_ms: timeout_ms.clamp(MIN_LOSS_TIMEOUT_MS, MAX_LOSS_TIMEOUT_MS),
                    behavior,
                },
            };
        }

        let dhcp = block[idx] != 0;
        idx += 1;
        let static_ip: [u8; 4] = block[idx..idx + 4].try_into().ok()?;
        idx += 4;
        let static_prefix = match block[idx] {
            p @ 1..=32 => p,
            _ => 24,
        };
        idx += 1;
        let has_gateway = block[idx] != 0;
        let static_gateway = if has_gateway {
            Some(block[idx + 1..idx + 5].try_into().ok()?)
        } else {
            None
        };
        idx += 5;

        let host_len = block[idx].min(32);
        idx += 1;
        let mut host_bytes = [0u8; 32];
        host_bytes.copy_from_slice(&block[idx..idx + 32]);

        Some(Self {
            pwm_freq_hz,
            channels,
            net: NetConfig {
                dhcp,
                static_ip,
                static_prefix,
                static_gateway,
            },
            hostname: core::str::from_utf8(&host_bytes[..host_len as usize])
                .map(CustomHostname::from_str)
                .unwrap_or(CustomHostname::empty()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_check() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn roundtrip_default_config() {
        let original = AppConfig::default();
        let mut block = [0u8; CONFIG_BLOCK_SIZE];
        original.encode(&mut block);

        assert_eq!(&block[0..4], b"HYDR");
        assert_eq!(u16::from_le_bytes([block[4], block[5]]), CONFIG_VERSION);

        let decoded = AppConfig::decode(&block).expect("failed to decode valid block");
        assert_eq!(original, decoded);
    }

    #[test]
    fn roundtrip_custom_config() {
        let mut original = AppConfig::default();
        original.pwm_freq_hz = 2000;
        original.channels[1].mode = ChannelMode::Ws2812 {
            universe: 42,
            start_channel: 100,
            pixel_count: 50,
        };
        original.channels[2].mode = ChannelMode::Pwm {
            universe: 5,
            dmx_channel: 12,
        };
        original.channels[3].mode = ChannelMode::Fire {
            pixel_count: 60,
            control: DmxSlot {
                universe: 10,
                channel: 1,
            },
        };
        original.channels[3].loss.behavior = LossBehavior::Fade { ms: 500 };
        original.net.dhcp = false;
        original.net.static_ip = [192, 168, 1, 99];
        original.net.static_prefix = 24;
        original.net.static_gateway = Some([192, 168, 1, 1]);
        original.hostname = CustomHostname::from_str("my-stage-lights");

        let mut block = [0u8; CONFIG_BLOCK_SIZE];
        original.encode(&mut block);

        let decoded = AppConfig::decode(&block).expect("failed to decode valid block");
        assert_eq!(original, decoded);
        assert_eq!(decoded.hostname.as_str(), "my-stage-lights");
    }

    #[test]
    fn rejects_erased_flash() {
        let erased = [0xFFu8; CONFIG_BLOCK_SIZE];
        assert!(AppConfig::decode(&erased).is_none());
    }

    #[test]
    fn rejects_corrupted_crc() {
        let original = AppConfig::default();
        let mut block = [0u8; CONFIG_BLOCK_SIZE];
        original.encode(&mut block);

        // Corrupt one payload byte
        block[20] ^= 0x55;
        assert!(AppConfig::decode(&block).is_none());
    }

    #[test]
    fn rejects_wrong_magic() {
        let original = AppConfig::default();
        let mut block = [0u8; CONFIG_BLOCK_SIZE];
        original.encode(&mut block);

        block[0] = b'X';
        assert!(AppConfig::decode(&block).is_none());
    }

    #[test]
    fn rejects_wrong_version() {
        let original = AppConfig::default();
        let mut block = [0u8; CONFIG_BLOCK_SIZE];
        original.encode(&mut block);

        block[4] = 99;
        assert!(AppConfig::decode(&block).is_none());
    }

    #[test]
    fn hostname_is_sanitized() {
        assert_eq!(
            CustomHostname::from_str("  My Host_1! ").as_str(),
            "myhost1"
        );
        assert_eq!(CustomHostname::from_str("-a-b-").as_str(), "a-b");
        assert_eq!(CustomHostname::from_str("<script>").as_str(), "script");
        assert_eq!(CustomHostname::from_str("").as_str(), "hydra");
        assert_eq!(CustomHostname::from_str("äöü").as_str(), "hydra");
        let long = "a".repeat(64);
        assert_eq!(CustomHostname::from_str(&long).len, 32);
    }

    #[test]
    fn decode_clamps_out_of_range_values() {
        let mut cfg = AppConfig::default();
        cfg.channels[1].mode = ChannelMode::Ws2812 {
            universe: 0,
            start_channel: 9999,
            pixel_count: 0,
        };
        cfg.net.static_prefix = 0;
        cfg.channels[1].loss.timeout_ms = 0;
        let mut block = [0u8; CONFIG_BLOCK_SIZE];
        cfg.encode(&mut block);
        let d = AppConfig::decode(&block).unwrap();
        assert_eq!(
            d.channels[1].mode,
            ChannelMode::Ws2812 {
                universe: 1,
                start_channel: 512,
                pixel_count: 1
            }
        );
        assert_eq!(d.net.static_prefix, 24);
        assert_eq!(d.channels[1].loss.timeout_ms, MIN_LOSS_TIMEOUT_MS);
    }

    #[test]
    fn static_net_validation() {
        assert!(validate_static_net([192, 168, 1, 50], 24, Some([192, 168, 1, 1])).is_ok());
        assert!(validate_static_net([192, 168, 1, 50], 24, None).is_ok());
        assert!(validate_static_net([0, 0, 0, 0], 24, None).is_err());
        assert!(validate_static_net([192, 168, 1, 255], 24, None).is_err());
        assert!(validate_static_net([192, 168, 1, 0], 24, None).is_err());
        assert!(validate_static_net([192, 168, 1, 50], 24, Some([10, 0, 0, 1])).is_err());
        assert!(validate_static_net([192, 168, 1, 50], 0, None).is_err());
        assert!(validate_static_net([224, 0, 0, 5], 24, None).is_err());
    }

    #[test]
    fn loss_timeout_ignores_off_channels() {
        let mut cfg = AppConfig::default();
        cfg.channels[0].mode = ChannelMode::Pwm {
            universe: 1,
            dmx_channel: 1,
        };
        cfg.channels[0].loss.timeout_ms = 4000;
        cfg.channels[3].loss.timeout_ms = 9000; // Off -> ignored
        assert_eq!(cfg.loss_timeout_ms(), 4000);
    }
}
