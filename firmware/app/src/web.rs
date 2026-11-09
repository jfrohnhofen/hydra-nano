//! Status and configuration web UI served over HTTP.
//!
//! Exposes:
//! - `GET /` -> Status & unified configuration page.
//! - `GET /api/status` -> Live status JSON polled by the page.
//! - `POST /network` -> Apply network settings (also persists the whole config).
//! - `POST /api/channel` -> Live-edit one output (mode, DMX mapping, loss behavior).
//! - `POST /api/pwm_freq`, `POST /api/loss` -> Live-edit global settings.
//! - `POST /api/outputs/save` -> Persist the live configuration to flash.
//! - `POST /api/outputs/revert` -> Discard unsaved changes (reload from flash).
//! - `POST /api/test/channel`, `POST /api/test/all` -> Interactive test patterns.

use core::sync::atomic::Ordering::Relaxed;
use embassy_time::Instant;
use heapless::String;
use picoserve::response::chunked::{ChunkWriter, ChunkedResponse, Chunks, ChunksWritten};
use picoserve::response::IntoResponse;
use picoserve::routing::{get, post};
use sacn_core::channel::{universe_range, ChannelMode, DmxSlot};
use sacn_core::config::{
    validate_static_net, DEFAULT_FADE_MS, MAX_LOSS_TIMEOUT_MS, MIN_LOSS_TIMEOUT_MS,
};
use sacn_core::loss::{LossBehavior, LossConfig};
use sacn_core::web::{
    render_channel_row, render_channel_table_footer, render_channel_table_header,
    render_network_card, render_stats_grid, render_status_json_head, render_status_json_tail,
    render_status_json_universe, render_topbar, render_universe_row, render_universes_empty_row,
    render_universes_footer, render_universes_header, ChannelView, PageData, UniverseView,
};

use crate::state::{
    AppConfig, CustomHostname, NetConfig, NetUpdateCmd, APP_CONFIG, CHANNEL_COUNT, CONFIG_DIRTY,
    NET_UPDATE_CHANNEL, UNIVERSE_STORE,
};

/// Received universes plus any universe an output is configured for but that
/// has not been heard yet, each tagged with the outputs that consume it.
/// 32 tracked + up to 48 configured (8 outputs x 6 universes) fits in 80.
fn build_universe_views(cfg: &AppConfig) -> heapless::Vec<UniverseView, 80> {
    let now = Instant::now();
    let live_ms = cfg.loss_timeout_ms() as u64;
    let infos = UNIVERSE_STORE.lock(|s| s.borrow().get_info());

    let mut views: heapless::Vec<UniverseView, 80> = heapless::Vec::new();
    for u in &infos {
        let active = match u.last_received {
            Some(last) => now.saturating_duration_since(last).as_millis() <= live_ms,
            None => false,
        };
        let _ = views.push(UniverseView {
            universe: u.universe,
            sequence: u.sequence_number,
            packets: u.packets_received,
            active,
            terminated: u.terminated,
            used_by: 0,
        });
    }
    for (i, ch) in cfg.channels.iter().enumerate() {
        if let Some((first, last)) = universe_range(ch.mode) {
            for uni in first..=last {
                let pos = match views.iter().position(|v| v.universe == uni) {
                    Some(p) => p,
                    None => {
                        let silent = UniverseView {
                            universe: uni,
                            sequence: 0,
                            packets: 0,
                            active: false,
                            terminated: false,
                            used_by: 0,
                        };
                        if views.push(silent).is_err() {
                            continue;
                        }
                        views.len() - 1
                    }
                };
                views[pos].used_by |= 1 << i;
            }
        }
    }
    views.sort_unstable_by_key(|v| v.universe);
    views
}

fn test_modes() -> [u8; CHANNEL_COUNT] {
    let mut modes = [0u8; CHANNEL_COUNT];
    for (i, m) in modes.iter_mut().enumerate() {
        *m = crate::state::CHANNEL_TEST_MODES[i].load(Relaxed);
    }
    modes
}

pub struct StatusPage;

impl Chunks for StatusPage {
    fn content_type(&self) -> &'static str {
        "text/html; charset=utf-8"
    }

    async fn write_chunks<W: picoserve::io::Write>(
        self,
        mut writer: ChunkWriter<W>,
    ) -> Result<ChunksWritten, W::Error> {
        let mut buf: String<2048> = String::new();
        let cfg = { *APP_CONFIG.lock().await };
        let identity = *crate::state::IDENTITY.get().await;
        let uptime = Instant::now().as_secs();
        let net = crate::state::NET_STATUS.lock(|s| s.get());
        let uni_views = build_universe_views(&cfg);
        let tests = test_modes();

        let mut ch_views = [ChannelView {
            mode: ChannelMode::Off,
            loss_behavior: LossBehavior::Blackout,
            loss_timeout_ms: 2500,
            test_mode: 0,
        }; CHANNEL_COUNT];
        for i in 0..CHANNEL_COUNT {
            ch_views[i] = ChannelView {
                mode: cfg.channels[i].mode,
                loss_behavior: cfg.channels[i].loss.behavior,
                loss_timeout_ms: cfg.channels[i].loss.timeout_ms,
                test_mode: tests[i],
            };
        }

        let test_mode = if tests.iter().any(|&m| m != 0) { 1 } else { 0 };

        let page_data = PageData {
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs: uptime,
            hostname: cfg.hostname.as_str(),
            mac: net.mac,
            link_up: net.link_up,
            ip: net.ip,
            is_fallback: net.is_fallback,
            dhcp_enabled: cfg.net.dhcp,
            static_ip: cfg.net.static_ip,
            static_prefix: cfg.net.static_prefix,
            static_gateway: cfg.net.static_gateway,
            default_hostname: identity.hostname(),
            default_ip: identity.fallback_ip,
            test_mode,
            pwm_freq_hz: cfg.pwm_freq_hz,
            loss_timeout_ms: cfg.loss_timeout_ms(),
            unsaved: CONFIG_DIRTY.load(Relaxed),
            channels: ch_views,
            universes: &uni_views,
        };

        // 1. Head & Styles (streamed directly from Flash)
        writer
            .write_chunk(sacn_core::web::HEAD_AND_STYLES.as_bytes())
            .await?;

        // 2. Topbar
        buf.clear();
        let _ = render_topbar(&mut buf, &page_data);
        writer.write_chunk(buf.as_bytes()).await?;

        // 3. Stats Grid
        buf.clear();
        let _ = render_stats_grid(&mut buf, &page_data);
        writer.write_chunk(buf.as_bytes()).await?;

        // 5. Output Channels Card Header
        buf.clear();
        let _ = render_channel_table_header(&mut buf, &page_data);
        writer.write_chunk(buf.as_bytes()).await?;

        for ch_idx in 0..CHANNEL_COUNT {
            buf.clear();
            let _ = render_channel_row(&mut buf, ch_idx, &ch_views[ch_idx]);
            writer.write_chunk(buf.as_bytes()).await?;
        }

        buf.clear();
        let _ = render_channel_table_footer(&mut buf);
        writer.write_chunk(buf.as_bytes()).await?;

        // 6. Network Settings Card
        buf.clear();
        let _ = render_network_card(&mut buf, &page_data);
        writer.write_chunk(buf.as_bytes()).await?;

        // 7. Universes Card (streamed row by row so the chunk buffer never overflows)
        buf.clear();
        let _ = render_universes_header(&mut buf);
        if uni_views.is_empty() {
            let _ = render_universes_empty_row(&mut buf);
        }
        writer.write_chunk(buf.as_bytes()).await?;
        for u in uni_views.iter() {
            buf.clear();
            let _ = render_universe_row(&mut buf, u);
            writer.write_chunk(buf.as_bytes()).await?;
        }
        buf.clear();
        let _ = render_universes_footer(&mut buf);
        writer.write_chunk(buf.as_bytes()).await?;

        // 8. Client Script (streamed directly from Flash)
        writer
            .write_chunk(sacn_core::web::SCRIPT.as_bytes())
            .await?;

        writer.finalize().await
    }
}

pub async fn get_status() -> ChunkedResponse<StatusPage> {
    defmt::info!("HTTP: GET / (Status Page)");
    ChunkedResponse::new(StatusPage)
}

pub struct StatusJson;

impl Chunks for StatusJson {
    fn content_type(&self) -> &'static str {
        "application/json"
    }

    async fn write_chunks<W: picoserve::io::Write>(
        self,
        mut writer: ChunkWriter<W>,
    ) -> Result<ChunksWritten, W::Error> {
        let uptime = Instant::now().as_secs();
        let net = crate::state::NET_STATUS.lock(|s| s.get());
        let cfg = { *APP_CONFIG.lock().await };
        let uni_views = build_universe_views(&cfg);
        let tests = test_modes();

        let mut buf: String<256> = String::new();
        let _ = render_status_json_head(
            &mut buf,
            uptime,
            net.link_up,
            net.ip.is_some(),
            CONFIG_DIRTY.load(Relaxed),
            &tests,
        );
        writer.write_chunk(buf.as_bytes()).await?;
        for (i, u) in uni_views.iter().enumerate() {
            buf.clear();
            let _ = render_status_json_universe(&mut buf, i == 0, u);
            writer.write_chunk(buf.as_bytes()).await?;
        }
        buf.clear();
        let _ = render_status_json_tail(&mut buf);
        writer.write_chunk(buf.as_bytes()).await?;
        writer.finalize().await
    }
}

pub async fn get_status_json() -> ChunkedResponse<StatusJson> {
    ChunkedResponse::new(StatusJson)
}

fn default_one() -> u16 {
    1
}
fn default_thirty() -> u16 {
    30
}

#[derive(serde::Deserialize, Default)]
pub struct NetworkForm {
    #[serde(default)]
    pub hostname: heapless::String<32>,
    #[serde(default)]
    pub dhcp: heapless::String<8>,
    #[serde(default)]
    pub ip: heapless::String<16>,
    /// Absent when the (disabled) static-IP inputs were not submitted.
    #[serde(default)]
    pub prefix: Option<u8>,
}

#[derive(serde::Deserialize, Default)]
pub struct PwmFreqForm {
    #[serde(default)]
    pub freq: u32,
    #[serde(default)]
    pub pwm_freq: u32,
}

/// Builds the new mode/loss for an output from the UI form. The stored loss
/// timeout and fade time are kept; the form only picks the behavior.
fn parse_channel(
    prev_loss: LossConfig,
    mode_str: &str,
    uni: u16,
    dmx: u16,
    px: u16,
    loss_str: &str,
) -> (ChannelMode, LossConfig) {
    let mode = match mode_str {
        "ws2812" => ChannelMode::Ws2812 {
            universe: uni.clamp(1, 63999),
            start_channel: dmx.clamp(1, 512),
            pixel_count: px.clamp(1, 1000),
        },
        "ws2812fill" => ChannelMode::Ws2812Fill {
            universe: uni.clamp(1, 63999),
            // Fill reads 3 slots (R, G, B) so it must start at 510 or lower.
            start_channel: dmx.clamp(1, 510),
            pixel_count: px.clamp(1, 1000),
        },
        "fire" => ChannelMode::Fire {
            pixel_count: px.clamp(1, 1000),
            // Dimmer, speed and color balance: 3 slots, so the start is capped at 510.
            control: DmxSlot {
                universe: uni.clamp(1, 63999),
                channel: dmx.clamp(1, 510),
            },
        },
        "pwm" => ChannelMode::Pwm {
            universe: uni.clamp(1, 63999),
            dmx_channel: dmx.clamp(1, 512),
        },
        _ => ChannelMode::Off,
    };

    let fade_ms = match prev_loss.behavior {
        LossBehavior::Fade { ms } => ms,
        _ => DEFAULT_FADE_MS,
    };
    let behavior = match loss_str {
        "hold" => LossBehavior::Hold,
        "fade" => LossBehavior::Fade { ms: fade_ms },
        _ => LossBehavior::Blackout,
    };
    let loss = LossConfig {
        timeout_ms: prev_loss.timeout_ms,
        behavior,
    };

    (mode, loss)
}

fn apply_net_html(target: &str) -> ([(&'static str, &'static str); 2], heapless::String<2000>) {
    let mut html = heapless::String::<2000>::new();
    use core::fmt::Write;
    let _ = write!(
        html,
        "<!doctype html><html><head><meta charset=\"utf-8\">\
        <meta http-equiv=\"refresh\" content=\"2; url=http://{target}/\">\
        <title>Applying Network Settings &bull; hydra nano</title>\
        <style>:root{{color-scheme:dark}}body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#090d16;color:#f8fafc;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}}\
        .card{{background:#111827;padding:2rem;border-radius:12px;border:1px solid #1f293d;text-align:center;max-width:440px;box-shadow:0 10px 25px rgba(0,0,0,0.5)}}\
        .spinner{{width:36px;height:36px;border:3px solid #1f293d;border-top-color:#38bdf8;border-radius:50%;animation:spin 1s linear infinite;margin:1.5rem auto}}\
        @keyframes spin{{to{{transform:rotate(360deg)}}}}\
        h1{{font-size:1.25rem;margin:0 0 0.5rem;color:#38bdf8}}\
        p{{font-size:0.9rem;color:#94a3b8;line-height:1.5}}\
        .btn{{display:inline-block;margin-top:1.25rem;padding:0.6rem 1.25rem;background:#0284c7;color:#fff;border-radius:6px;text-decoration:none;font-weight:600;font-size:0.85rem}}\
        .btn:hover{{background:#0369a1}}</style>\
        <script>setTimeout(function(){{location.href=\"http://{target}/\";}},2000);</script>\
        </head><body><div class=\"card\">\
        <h1>Applying Settings</h1>\
        <div class=\"spinner\"></div>\
        <p>Configuring network interface...<br>Redirecting to <strong>http://{target}/</strong></p>\
        <a class=\"btn\" href=\"http://{target}/\">Click here if not redirected</a>\
        </div></body></html>"
    );
    (
        [
            ("Content-Type", "text/html; charset=utf-8"),
            ("Cache-Control", "no-cache, no-store, must-revalidate"),
        ],
        html,
    )
}

fn net_error_html(msg: &str) -> ([(&'static str, &'static str); 2], heapless::String<2000>) {
    let mut html = heapless::String::<2000>::new();
    use core::fmt::Write;
    let _ = write!(
        html,
        "<!doctype html><html><head><meta charset=\"utf-8\">\
        <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
        <title>Network settings rejected</title>\
        <style>:root{{color-scheme:dark}}body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#090d16;color:#f8fafc;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}}\
        .card{{background:#111827;padding:2rem;border-radius:12px;border:1px solid #7f1d1d;text-align:center;max-width:440px}}\
        h1{{font-size:1.25rem;margin:0 0 0.5rem;color:#f87171}}p{{font-size:0.9rem;color:#94a3b8}}\
        a{{display:inline-block;margin-top:1rem;padding:0.6rem 1.25rem;background:#facc15;color:#0f172a;border-radius:6px;text-decoration:none;font-weight:700}}</style>\
        </head><body><div class=\"card\"><h1>Settings not applied</h1><p>{msg}</p>\
        <a href=\"/\">Back</a></div></body></html>"
    );
    (
        [
            ("Content-Type", "text/html; charset=utf-8"),
            ("Cache-Control", "no-store"),
        ],
        html,
    )
}

pub async fn post_network(
    picoserve::extract::Form(form): picoserve::extract::Form<NetworkForm>,
) -> impl IntoResponse {
    let mut net_changed = false;
    let mut net_cmd = None;
    let mut redirect_target: Option<heapless::String<64>> = None;

    let cfg_to_save = {
        let mut cfg = APP_CONFIG.lock().await;

        let dhcp = form.dhcp.as_str() == "true";
        let parsed_ip = parse_ipv4(form.ip.as_str());
        if parsed_ip.is_none() && !form.ip.is_empty() {
            return Ok(net_error_html("That is not a valid IPv4 address."));
        }
        let static_ip = parsed_ip.unwrap_or(cfg.net.static_ip);
        let static_prefix = match form.prefix {
            Some(p) if (1..=32).contains(&p) => p,
            _ => cfg.net.static_prefix,
        };
        // Gateway is not configurable from the UI; clear any stale value.
        let static_gateway = None;
        // The address is the static IP, or the fallback when DHCP is on: either way it must be usable.
        if let Err(msg) = validate_static_net(static_ip, static_prefix, static_gateway) {
            return Ok(net_error_html(msg));
        }
        // The hostname is reduced to a valid DNS label; if nothing usable is
        // left, keep the current one rather than falling back to a generic name.
        let custom_host = {
            let h = CustomHostname::from_str(form.hostname.as_str());
            if h.len == 0 {
                cfg.hostname
            } else {
                h
            }
        };

        if cfg.net.dhcp != dhcp
            || cfg.net.static_ip != static_ip
            || cfg.net.static_prefix != static_prefix
            || cfg.net.static_gateway != static_gateway
            || cfg.hostname != custom_host
        {
            net_changed = true;
            net_cmd = Some(NetUpdateCmd {
                dhcp,
                static_ip,
                static_prefix,
                static_gateway,
                hostname: custom_host,
            });

            if !dhcp {
                let mut s = heapless::String::<64>::new();
                use core::fmt::Write;
                let _ = write!(
                    s,
                    "{}.{}.{}.{}",
                    static_ip[0], static_ip[1], static_ip[2], static_ip[3]
                );
                redirect_target = Some(s);
            } else {
                let mut s = heapless::String::<64>::new();
                use core::fmt::Write;
                let _ = write!(s, "{}.local", custom_host.as_str());
                redirect_target = Some(s);
            }
        }

        cfg.net = NetConfig {
            dhcp,
            static_ip,
            static_prefix,
            static_gateway,
        };
        cfg.hostname = custom_host;

        *cfg
    };

    defmt::info!(
        "HTTP: POST /network -> host {} dhcp {}",
        cfg_to_save.hostname.as_str(),
        cfg_to_save.net.dhcp
    );
    // Saving persists the whole live configuration, including output edits.
    if crate::flash::save_config(&cfg_to_save).await.is_ok() {
        CONFIG_DIRTY.store(false, Relaxed);
    }

    if net_changed {
        if let Some(cmd) = net_cmd {
            let _ = NET_UPDATE_CHANNEL.try_send(cmd);
        }
        if let Some(target) = redirect_target {
            return Ok(apply_net_html(target.as_str()));
        }
    }

    Err(picoserve::response::Redirect::to("/"))
}

pub async fn post_pwm_freq(
    picoserve::extract::Form(form): picoserve::extract::Form<PwmFreqForm>,
) -> impl IntoResponse {
    let freq = if form.freq > 0 {
        form.freq
    } else {
        form.pwm_freq
    };
    defmt::info!("HTTP: POST /api/pwm_freq -> {} Hz", freq);
    if freq >= sacn_core::channel::MIN_PWM_FREQ_HZ && freq <= sacn_core::channel::MAX_PWM_FREQ_HZ {
        let mut cfg = APP_CONFIG.lock().await;
        if cfg.pwm_freq_hz != freq {
            cfg.pwm_freq_hz = freq;
            CONFIG_DIRTY.store(true, Relaxed);
        }
    }
    "ok"
}

#[derive(serde::Deserialize, Default)]
pub struct LossForm {
    #[serde(default)]
    pub timeout: u32,
}

/// Sets the signal-loss timeout for all outputs.
pub async fn post_loss(
    picoserve::extract::Form(form): picoserve::extract::Form<LossForm>,
) -> impl IntoResponse {
    if form.timeout > 0 {
        let timeout = form.timeout.clamp(MIN_LOSS_TIMEOUT_MS, MAX_LOSS_TIMEOUT_MS);
        defmt::info!("HTTP: POST /api/loss -> {} ms", timeout);
        let mut cfg = APP_CONFIG.lock().await;
        let mut changed = false;
        for ch in cfg.channels.iter_mut() {
            if ch.loss.timeout_ms != timeout {
                ch.loss.timeout_ms = timeout;
                changed = true;
            }
        }
        if changed {
            CONFIG_DIRTY.store(true, Relaxed);
        }
    }
    "ok"
}

#[derive(serde::Deserialize)]
pub struct ChannelForm {
    pub ch: usize,
    #[serde(default)]
    pub mode: heapless::String<16>,
    #[serde(default = "default_one")]
    pub universe: u16,
    #[serde(default = "default_one")]
    pub dmx_channel: u16,
    #[serde(default = "default_thirty")]
    pub pixel_count: u16,
    #[serde(default)]
    pub loss: heapless::String<16>,
}

pub async fn post_channel(
    picoserve::extract::Form(form): picoserve::extract::Form<ChannelForm>,
) -> impl IntoResponse {
    if form.ch >= 1 && form.ch <= CHANNEL_COUNT {
        let ch_idx = form.ch - 1;
        defmt::info!(
            "HTTP: POST /api/channel -> ch {} mode {} uni {} dmx {} px {} loss {}",
            form.ch,
            form.mode.as_str(),
            form.universe,
            form.dmx_channel,
            form.pixel_count,
            form.loss.as_str()
        );
        let mut cfg = APP_CONFIG.lock().await;
        let (mode, loss) = parse_channel(
            cfg.channels[ch_idx].loss,
            form.mode.as_str(),
            form.universe,
            form.dmx_channel,
            form.pixel_count,
            form.loss.as_str(),
        );
        if cfg.channels[ch_idx].mode != mode || cfg.channels[ch_idx].loss != loss {
            cfg.channels[ch_idx].mode = mode;
            cfg.channels[ch_idx].loss = loss;
            CONFIG_DIRTY.store(true, Relaxed);
        }
    }
    "ok"
}

pub async fn post_save_outputs() -> impl IntoResponse {
    defmt::info!("HTTP: POST /api/outputs/save -> writing configuration to Flash");
    let cfg = {
        let cfg_guard = APP_CONFIG.lock().await;
        *cfg_guard
    };
    match crate::flash::save_config(&cfg).await {
        Ok(()) => {
            defmt::info!("Flash save OK");
            CONFIG_DIRTY.store(false, Relaxed);
            (picoserve::response::StatusCode::OK, "ok")
        }
        Err(()) => {
            defmt::error!("Flash save FAILED");
            (
                picoserve::response::StatusCode::INTERNAL_SERVER_ERROR,
                "error",
            )
        }
    }
}

/// Discards all unsaved channel/PWM/loss changes by reloading them from flash.
/// Network settings are applied and saved immediately, so they are left alone.
pub async fn post_revert_outputs() -> impl IntoResponse {
    defmt::info!("HTTP: POST /api/outputs/revert -> restoring saved configuration");
    let saved = crate::flash::load_config()
        .await
        .unwrap_or(sacn_core::config::AppConfig::const_default());
    {
        let mut cfg = APP_CONFIG.lock().await;
        cfg.pwm_freq_hz = saved.pwm_freq_hz;
        cfg.channels = saved.channels;
    }
    CONFIG_DIRTY.store(false, Relaxed);
    "ok"
}

pub async fn post_save_outputs_form() -> impl IntoResponse {
    defmt::info!("HTTP: POST /outputs/save -> writing configuration to Flash");
    let cfg = {
        let cfg_guard = APP_CONFIG.lock().await;
        *cfg_guard
    };
    if crate::flash::save_config(&cfg).await.is_ok() {
        CONFIG_DIRTY.store(false, Relaxed);
    }
    picoserve::response::Redirect::to("/")
}

pub fn parse_ipv4(s: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut count = 0;
    for part in s.trim().split('.') {
        if count >= 4 {
            return None;
        }
        let val = part.parse::<u8>().ok()?;
        octets[count] = val;
        count += 1;
    }
    if count == 4 {
        Some(octets)
    } else {
        None
    }
}

#[derive(serde::Deserialize, Default)]
pub struct ChannelTestForm {
    #[serde(default)]
    pub ch: usize,
    #[serde(default)]
    pub test: u8,
    #[serde(default)]
    pub mode: u8,
    #[serde(default)]
    pub ch_mode: heapless::String<16>,
    #[serde(default)]
    pub pixels: Option<u16>,
}

pub async fn post_channel_test(
    picoserve::extract::Form(form): picoserve::extract::Form<ChannelTestForm>,
) -> impl IntoResponse {
    if form.ch >= 1 && form.ch <= CHANNEL_COUNT {
        let raw_mode = if form.test != 0 { form.test } else { form.mode };
        defmt::info!(
            "HTTP: POST /api/test/channel -> ch {} test_mode {}",
            form.ch,
            raw_mode
        );
        let mut pattern = sacn_core::test_pattern::TestPattern::from_u8(raw_mode);
        let ch_idx = form.ch - 1;

        let ch_mode_str = form.ch_mode.as_str();
        let applicable = match ch_mode_str {
            "ws2812" | "ws2812fill" | "fire" => true,
            "pwm" => matches!(
                pattern,
                sacn_core::test_pattern::TestPattern::Off
                    | sacn_core::test_pattern::TestPattern::FullWhite
                    | sacn_core::test_pattern::TestPattern::Breathe
            ),
            "off" => false,
            _ => true,
        };
        if !applicable {
            pattern = sacn_core::test_pattern::TestPattern::Off;
        }

        crate::state::CHANNEL_TEST_MODES[ch_idx]
            .store(pattern.as_u8(), core::sync::atomic::Ordering::Relaxed);

        if pattern == sacn_core::test_pattern::TestPattern::Off {
            crate::state::CHANNEL_TEST_PIXELS[ch_idx]
                .store(0, core::sync::atomic::Ordering::Relaxed);
        } else if let Some(px) = form.pixels {
            if px > 0 {
                crate::state::CHANNEL_TEST_PIXELS[ch_idx]
                    .store(px, core::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    "ok"
}

#[derive(serde::Deserialize, Default)]
pub struct TestForm {
    #[serde(default)]
    pub mode: u8,
}

/// Sets (or, with mode 0, clears) the test pattern on every output.
pub async fn post_test_all(
    picoserve::extract::Form(form): picoserve::extract::Form<TestForm>,
) -> impl IntoResponse {
    let pattern = sacn_core::test_pattern::TestPattern::from_u8(form.mode);
    for i in 0..CHANNEL_COUNT {
        crate::state::CHANNEL_TEST_MODES[i]
            .store(pattern.as_u8(), core::sync::atomic::Ordering::Relaxed);
        if pattern == sacn_core::test_pattern::TestPattern::Off {
            crate::state::CHANNEL_TEST_PIXELS[i].store(0, core::sync::atomic::Ordering::Relaxed);
        }
    }
    "ok"
}

pub fn router() -> picoserve::Router<impl picoserve::routing::PathRouter> {
    picoserve::Router::new()
        .route("/", get(get_status))
        .route("/api/status", get(get_status_json))
        .route("/api/test/channel", post(post_channel_test))
        .route("/api/test/all", post(post_test_all))
        .route("/api/channel", post(post_channel))
        .route("/api/pwm_freq", post(post_pwm_freq))
        .route("/api/loss", post(post_loss))
        .route("/api/outputs/save", post(post_save_outputs))
        .route("/api/outputs/revert", post(post_revert_outputs))
        // Plain-form fallback for the Save button when JavaScript is unavailable.
        .route("/outputs/save", post(post_save_outputs_form))
        .route("/network", post(post_network))
}
