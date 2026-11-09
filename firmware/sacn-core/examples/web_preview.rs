//! Host-side web preview server for rapid UI development and styling.
//!
//! Run with: `cargo run -p sacn-core --example web_preview`
//! Then open: `http://127.0.0.1:8080/` in your browser.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::Instant;

use sacn_core::channel::{ChannelMode, DmxSlot};
use sacn_core::loss::LossBehavior;

use sacn_core::web::{
    render_channel_row, render_channel_table_footer, render_channel_table_header,
    render_head_and_styles, render_network_card, render_script, render_stats_grid, render_topbar,
    render_universes_card, ChannelView, PageData, UniverseView, CHANNEL_COUNT,
};

struct ServerState {
    start_time: Instant,
    hostname: String,
    dhcp_enabled: bool,
    static_ip: [u8; 4],
    static_prefix: u8,
    static_gateway: Option<[u8; 4]>,
    test_mode: u8,
    pwm_freq_hz: u32,
    loss_timeout_ms: u32,
    unsaved: bool,
    saved_pwm_freq_hz: u32,
    saved_channels: [ChannelView; CHANNEL_COUNT],
    channels: [ChannelView; CHANNEL_COUNT],
    universes: Vec<UniverseView>,
}

static STATE: Mutex<Option<ServerState>> = Mutex::new(None);

fn main() {
    let mut initial_channels = [ChannelView {
        mode: ChannelMode::Off,
        loss_behavior: LossBehavior::Blackout,
        loss_timeout_ms: 2500,
        test_mode: 0,
    }; CHANNEL_COUNT];

    initial_channels[0].mode = ChannelMode::Fire {
        pixel_count: 30,
        control: DmxSlot {
            universe: 1,
            channel: 1,
        },
    };
    initial_channels[1].mode = ChannelMode::Ws2812 {
        universe: 1,
        start_channel: 1,
        pixel_count: 30,
    };

    let initial_universes = vec![
        UniverseView {
            universe: 1,
            sequence: 42,
            packets: 1250,
            active: true,
            terminated: false,
            used_by: 0,
        },
        UniverseView {
            universe: 2,
            sequence: 18,
            packets: 340,
            active: false,
            terminated: false,
            used_by: 0,
        },
    ];

    *STATE.lock().unwrap() = Some(ServerState {
        start_time: Instant::now(),
        hostname: "hydra-3498cb".to_string(),
        dhcp_enabled: true,
        static_ip: [169, 254, 246, 176],
        static_prefix: 16,
        static_gateway: None,
        test_mode: 0,
        pwm_freq_hz: 1000,
        loss_timeout_ms: 2500,
        unsaved: false,
        saved_pwm_freq_hz: 1000,
        saved_channels: initial_channels,
        channels: initial_channels,
        universes: initial_universes,
    });

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).expect("failed to bind address");
    println!("\n🧪 hydra nano Web Preview running on http://{}\n", addr);

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            handle_connection(stream);
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let req = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = req.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

    if method == "POST" {
        let body = req.split("\r\n\r\n").nth(1).unwrap_or("");
        handle_post(path, body);
        let resp = if path.starts_with("/api/") {
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
        } else {
            "HTTP/1.1 303 See Other\r\nLocation: /\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        };
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if path == "/api/status" {
        let mut state_guard = STATE.lock().unwrap();
        let state = state_guard.as_mut().unwrap();
        let uptime = state.start_time.elapsed().as_secs();
        let mut json = String::new();
        let tests = state.channels.map(|c| c.test_mode);
        let unis = universe_views(state);
        let _ = sacn_core::web::render_status_json(
            &mut json,
            uptime,
            true,
            true,
            state.unsaved,
            &tests,
            &unis,
        );
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            json.len(),
            json
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    // GET / -> render page
    let mut state_guard = STATE.lock().unwrap();
    let state = state_guard.as_mut().unwrap();
    let uptime = state.start_time.elapsed().as_secs();

    let unis = universe_views(state);
    let page_data = PageData {
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: uptime,
        hostname: &state.hostname,
        mac: [0x12, 0x08, 0x8f, 0x34, 0x98, 0xcb],
        link_up: true,
        ip: Some((state.static_ip, state.static_prefix)),
        is_fallback: state.dhcp_enabled,
        dhcp_enabled: state.dhcp_enabled,
        static_ip: state.static_ip,
        static_prefix: state.static_prefix,
        static_gateway: state.static_gateway,
        default_hostname: "hydra-3498cb",
        default_ip: [169, 254, 246, 176],
        test_mode: state.test_mode,
        pwm_freq_hz: state.pwm_freq_hz,
        loss_timeout_ms: state.loss_timeout_ms,
        unsaved: state.unsaved,
        channels: state.channels,
        universes: &unis,
    };

    let mut html = String::new();
    let _ = render_head_and_styles(&mut html);
    let _ = render_topbar(&mut html, &page_data);
    let _ = render_stats_grid(&mut html, &page_data);
    let _ = render_channel_table_header(&mut html, &page_data);

    for ch_idx in 0..CHANNEL_COUNT {
        let _ = render_channel_row(&mut html, ch_idx, &state.channels[ch_idx]);
    }
    let _ = render_channel_table_footer(&mut html);

    let _ = render_network_card(&mut html, &page_data);
    let _ = render_universes_card(&mut html, &unis);
    let _ = render_script(&mut html);

    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    let _ = stream.write_all(resp.as_bytes());
}

/// Mirrors the firmware: received universes plus configured-but-silent ones,
/// each tagged with the outputs that consume it.
fn universe_views(state: &ServerState) -> Vec<UniverseView> {
    let mut list = state.universes.clone();
    for (i, ch) in state.channels.iter().enumerate() {
        if let Some((a, b)) = sacn_core::channel::universe_range(ch.mode) {
            for u in a..=b {
                if !list.iter().any(|v| v.universe == u) {
                    list.push(UniverseView {
                        universe: u,
                        sequence: 0,
                        packets: 0,
                        active: false,
                        terminated: false,
                        used_by: 0,
                    });
                }
                list.iter_mut().find(|v| v.universe == u).unwrap().used_by |= 1 << i;
            }
        }
    }
    for v in list.iter_mut() {
        if !state.channels.iter().any(|c| {
            sacn_core::channel::universe_range(c.mode)
                .is_some_and(|(a, b)| (a..=b).contains(&v.universe))
        }) {
            v.used_by = 0;
        }
    }
    list.sort_by_key(|v| v.universe);
    list
}

fn handle_post(path: &str, body: &str) {
    let mut params = std::collections::HashMap::new();
    for pair in body.split('&') {
        let mut kv = pair.split('=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            params.insert(k, v);
        }
    }

    let mut state_guard = STATE.lock().unwrap();
    let state = state_guard.as_mut().unwrap();

    match path {
        "/api/pwm_freq" => {
            if let Some(freq_str) = params.get("freq").or_else(|| params.get("pwm_freq")) {
                if let Ok(freq) = freq_str.parse::<u32>() {
                    state.pwm_freq_hz = freq.clamp(100, 20000);
                    state.unsaved = true;
                    println!("Global PWM freq updated: {} Hz", state.pwm_freq_hz);
                }
            }
        }
        "/api/loss" => {
            if let Some(ms) = params.get("timeout").and_then(|v| v.parse::<u32>().ok()) {
                state.loss_timeout_ms = ms.clamp(100, 60_000);
                for ch in &mut state.channels {
                    ch.loss_timeout_ms = state.loss_timeout_ms;
                }
                state.unsaved = true;
            }
        }
        "/api/channel" | "/channel" => {
            let ch_num: usize = params.get("ch").and_then(|v| v.parse().ok()).unwrap_or(1);
            if ch_num >= 1 && ch_num <= CHANNEL_COUNT {
                let ch_idx = ch_num - 1;
                let mode_str = params.get("mode").copied().unwrap_or("off");
                let uni: u16 = params
                    .get("universe")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let dmx: u16 = params
                    .get("dmx_channel")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let px: u16 = params
                    .get("pixel_count")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30);
                let loss_str = params.get("loss").copied().unwrap_or("blackout");

                let mode = match mode_str {
                    "ws2812" => ChannelMode::Ws2812 {
                        universe: uni,
                        start_channel: dmx,
                        pixel_count: px,
                    },
                    "ws2812fill" => ChannelMode::Ws2812Fill {
                        universe: uni,
                        start_channel: dmx,
                        pixel_count: px,
                    },
                    "fire" => ChannelMode::Fire {
                        pixel_count: px,
                        control: DmxSlot {
                            universe: uni,
                            channel: dmx.clamp(1, 510),
                        },
                    },
                    "pwm" => ChannelMode::Pwm {
                        universe: uni,
                        dmx_channel: dmx,
                    },
                    _ => ChannelMode::Off,
                };
                let loss_behavior = match loss_str {
                    "hold" => LossBehavior::Hold,
                    "fade" => LossBehavior::Fade { ms: 1000 },
                    _ => LossBehavior::Blackout,
                };
                let prev_test = state.channels[ch_idx].test_mode;
                state.channels[ch_idx] = ChannelView {
                    mode,
                    loss_behavior,
                    loss_timeout_ms: 2500,
                    test_mode: prev_test,
                };
                state.channels[ch_idx].loss_timeout_ms = state.loss_timeout_ms;
                state.unsaved = true;
                println!("Channel {} updated: {:?}", ch_num, mode);
            }
        }
        "/api/outputs/save" | "/outputs/save" => {
            state.unsaved = false;
            state.saved_pwm_freq_hz = state.pwm_freq_hz;
            state.saved_channels = state.channels;
            println!("Outputs configuration persisted to flash!");
        }
        "/api/outputs/revert" => {
            state.unsaved = false;
            state.pwm_freq_hz = state.saved_pwm_freq_hz;
            state.channels = state.saved_channels;
            println!("Pending changes discarded");
        }
        "/network" => {
            if let Some(host) = params.get("hostname") {
                state.hostname = host.to_string();
            }
            if let Some(dhcp) = params.get("dhcp") {
                state.dhcp_enabled = *dhcp == "true";
            }
            if let Some(ip_str) = params.get("ip") {
                let parts: Vec<u8> = ip_str.split('.').filter_map(|s| s.parse().ok()).collect();
                if parts.len() == 4 {
                    state.static_ip = [parts[0], parts[1], parts[2], parts[3]];
                }
            }
            if let Some(p_str) = params.get("prefix") {
                if let Ok(p) = p_str.parse::<u8>() {
                    state.static_prefix = p.clamp(1, 32);
                }
            }
            println!(
                "Network updated: hostname={}, dhcp={}",
                state.hostname, state.dhcp_enabled
            );
        }
        "/api/test/all" | "/test" => {
            let mode: u8 = params.get("mode").and_then(|v| v.parse().ok()).unwrap_or(0);
            state.test_mode = mode;
            for ch in &mut state.channels {
                ch.test_mode = mode;
            }
            println!("Test mode updated to {}", mode);
        }
        "/api/test/channel" => {
            let ch_num: usize = params.get("ch").and_then(|v| v.parse().ok()).unwrap_or(1);
            let mode: u8 = params
                .get("test")
                .or_else(|| params.get("mode"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            if ch_num >= 1 && ch_num <= CHANNEL_COUNT {
                state.channels[ch_num - 1].test_mode = mode;
                println!("Channel {} test mode updated to {}", ch_num, mode);
            }
        }
        _ => {}
    }
}
