use crate::channel::ChannelMode;
use crate::loss::LossBehavior;

pub const CHANNEL_COUNT: usize = 8;
pub const GROUP_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelView {
    pub mode: ChannelMode,
    pub loss_behavior: LossBehavior,
    pub loss_timeout_ms: u32,
    pub test_mode: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniverseView {
    pub universe: u16,
    pub sequence: u8,
    pub packets: u32,
    pub active: bool,
    pub terminated: bool,
    /// Bitmask of outputs (bit 0 = Output 1) configured to consume this universe.
    pub used_by: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct PageData<'a> {
    pub version: &'a str,
    pub uptime_secs: u64,
    pub hostname: &'a str,
    pub mac: [u8; 6],
    pub link_up: bool,
    pub ip: Option<([u8; 4], u8)>,
    pub is_fallback: bool,
    pub dhcp_enabled: bool,
    pub static_ip: [u8; 4],
    pub static_prefix: u8,
    pub static_gateway: Option<[u8; 4]>,
    /// Factory network identity, used by the client-side "Reset to defaults".
    pub default_hostname: &'a str,
    pub default_ip: [u8; 4],
    pub test_mode: u8,
    pub pwm_freq_hz: u32,
    pub loss_timeout_ms: u32,
    /// Live settings differ from what is stored in flash.
    pub unsaved: bool,
    pub channels: [ChannelView; CHANNEL_COUNT],
    pub universes: &'a [UniverseView],
}

pub const HEAD_AND_STYLES: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>hydra NANO &bull; sACN Controller</title>\
<link rel=\"icon\" href=\"data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32' fill='none' stroke='%23facc15' stroke-width='2.6' stroke-linecap='round'><path d='M16 29V20M16 20C16 14 7 15 7 7M16 20V7M16 20C16 14 25 15 25 7'/><g fill='%23facc15' stroke='none'><circle cx='7' cy='5.5' r='3'/><circle cx='16' cy='5.5' r='3'/><circle cx='25' cy='5.5' r='3'/></g></svg>\"><style>\
:root{color-scheme:dark;\
--bg:#090d16;--surface:#111827;--surface-card:#162032;--surface-hdr:#0d1421;\
--border:#26354a;--border-light:#334460;--text:#f1f5f9;--text-muted:#94a3b8;\
--primary:#facc15;--primary-hover:#eab308;--primary-text:#0f172a;\
--cyan:#38bdf8;--orange:#f97316;--green:#10b981;--red:#ef4444;--purple:#a855f7;\
}\
*{box-sizing:border-box}body{\
font-family:-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,sans-serif;\
background:var(--bg);color:var(--text);margin:0;padding:1.5rem;line-height:1.5;\
}\
.container{max-width:1160px;margin:0 auto}\
.topbar{display:flex;align-items:center;justify-content:space-between;padding:1rem 1.25rem;\
background:var(--surface);border:1px solid var(--border);border-radius:12px;margin-bottom:1.5rem;\
box-shadow:0 4px 12px rgba(0,0,0,0.3);flex-wrap:wrap;gap:1rem}\
.brand{display:flex;align-items:center;gap:0.75rem}\
.brand-logo{display:flex;color:var(--primary)}\
.brand-logo svg{width:38px;height:38px}\
.brand-title{font-size:1.35rem;font-weight:400;letter-spacing:-0.02em;margin:0;display:flex;align-items:baseline}\
.brand-ra{font-weight:800}\
.brand-nano{font-size:0.62em;font-weight:800;letter-spacing:0.06em;color:var(--primary);margin-left:0.5em}\
.brand-sub{font-size:0.8rem;color:var(--text-muted);margin:0}\
.topbar-pills{display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap}\
.pill{display:inline-flex;align-items:center;gap:0.4rem;padding:4px 10px;border-radius:9999px;\
font-size:0.75rem;font-weight:600;font-family:monospace;letter-spacing:0.02em;text-transform:lowercase}\
.pill-green{background:#064e3b;color:#34d399;border:1px solid #059669}\
.pill-red{background:#450a0a;color:#f87171;border:1px solid #dc2626}\
.pill-yellow{background:#422006;color:#facc15;border:1px solid #ca8a04}\
.pill-cyan{background:#0c4a6e;color:#38bdf8;border:1px solid #0284c7}\
.pill-gray{background:#1e293b;color:#94a3b8;border:1px solid var(--border)}\
.pulse{width:7px;height:7px;border-radius:50%;background:#34d399;animation:blink 1.8s infinite}\
@keyframes blink{0%,100%{opacity:1}50%{opacity:0.3}}\
.grid-stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:1rem;margin-bottom:1.5rem}\
.stat-card{background:var(--surface);border:1px solid var(--border);border-radius:10px;padding:0.9rem 1.15rem;\
box-shadow:0 2px 8px rgba(0,0,0,0.2);display:flex;flex-direction:column;gap:0.65rem;justify-content:center}\
.stat-item{display:flex;flex-direction:column;gap:0.2rem}\
.stat-label{font-size:0.68rem;text-transform:uppercase;color:var(--text-muted);letter-spacing:0.05em;font-weight:700}\
.stat-val{font-size:1rem;font-weight:700;color:#fff;word-break:break-all;display:flex;align-items:center;gap:0.45rem;flex-wrap:wrap}\
.stat-mono{font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;font-size:0.92rem;font-weight:600;letter-spacing:0.02em}\
.card{background:var(--surface);border:1px solid var(--border);border-radius:12px;margin-bottom:1.5rem;\
box-shadow:0 4px 12px rgba(0,0,0,0.25)}\
.card-title{font-size:0.95rem;font-weight:700;margin:0;display:flex;align-items:center;gap:0.4rem}\
.card-hdr{padding:0.8rem 1.25rem;border-bottom:1px solid var(--border);background:var(--surface-hdr);\
border-top-left-radius:11px;border-top-right-radius:11px;\
display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:0.5rem}\
.card-body{padding:1.25rem}\
.net-form{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:1rem;align-items:end}\
.f-grp label{display:block;font-size:0.75rem;font-weight:600;color:var(--text-muted);margin-bottom:0.35rem;text-transform:uppercase}\
input[type=\"text\"],input[type=\"number\"]{\
width:100%;background:var(--bg);color:var(--text);border:1px solid var(--border-light);\
border-radius:6px;padding:7px 10px;font-size:0.875rem;transition:border-color 0.15s,box-shadow 0.15s;\
}\
select{display:none!important}\
.c-select{position:relative;display:inline-block;width:100%}\
.c-select-btn{\
width:100%;background:var(--bg);color:var(--text);border:1px solid var(--border-light);\
border-radius:6px;padding:7px 32px 7px 10px;font-size:0.875rem;font-weight:500;cursor:pointer;\
display:flex;align-items:center;justify-content:space-between;text-align:left;position:relative;\
user-select:none;transition:border-color 0.15s,box-shadow 0.15s;outline:none;overflow:hidden;\
background-image:url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E\");\
background-repeat:no-repeat;background-position:right 10px center;\
}\
.c-select-btn span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;display:block}\
.c-select-btn:hover{border-color:var(--border-light);background-color:#111a2c}\
.c-select.open .c-select-btn,.c-select-btn:focus{border-color:var(--primary);box-shadow:0 0 0 2px rgba(250,204,21,0.25);\
background-image:url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23facc15' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='18 15 12 9 6 15'%3E%3C/polyline%3E%3C/svg%3E\")}\
.c-select-menu{\
display:none;position:absolute;top:calc(100% + 4px);left:0;min-width:100%;\
background:#111a2c;border:1px solid var(--border-light);border-radius:8px;\
box-shadow:0 12px 28px rgba(0,0,0,0.65);z-index:100;max-height:220px;overflow-y:auto;padding:4px;\
}\
.c-select.open .c-select-menu{display:block}\
.c-select-item{\
padding:7px 10px;border-radius:5px;font-size:0.85rem;color:var(--text);\
cursor:pointer;display:flex;align-items:center;justify-content:space-between;\
transition:background 0.1s,color 0.1s;white-space:nowrap;\
}\
.c-select-item:hover{background:#1e2d47;color:var(--primary)}\
.c-select-item.selected{background:#1e293b;color:var(--primary);font-weight:700}\
input:disabled{opacity:0.42;cursor:not-allowed;background:#0d121c;border-color:var(--border)}\
input[type=\"number\"]{\
font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;\
font-variant-numeric:tabular-nums;\
font-size:0.92rem;font-weight:600;text-align:center;\
-moz-appearance:textfield;letter-spacing:0.03em;\
}\
input[type=\"number\"]::-webkit-outer-spin-button,\
input[type=\"number\"]::-webkit-inner-spin-button{-webkit-appearance:none;margin:0}\
.btn{background:var(--primary);color:var(--primary-text);border:none;border-radius:6px;padding:7px 14px;\
font-size:0.85rem;font-weight:700;cursor:pointer;transition:all 0.15s;text-align:center}\
.btn:hover{background:var(--primary-hover);transform:translateY(-1px)}\
.btn-sm{padding:4px 10px;font-size:0.78rem}\
.btn-lg{padding:9px 24px;font-size:0.95rem}\
.table{width:100%;border-collapse:collapse;font-size:0.875rem;text-align:left}\
.table-fixed{table-layout:fixed}\
.table th{background:var(--surface-hdr);color:var(--text-muted);font-weight:600;padding:0.75rem 1rem;\
border-bottom:1px solid var(--border);font-size:0.78rem;text-transform:uppercase;letter-spacing:0.04em}\
.table td{padding:0.65rem 1rem;border-bottom:1px solid var(--border);vertical-align:middle;position:relative}\
.table tbody tr:hover td{background:#152238}\
.grp-hdr{background:#182640!important;border-top:10px solid var(--bg)!important;box-shadow:inset 0 1px 0 rgba(255,255,255,0.06)}\
.grp-hdr td{padding:0.75rem 1rem!important}\
.grp-hdr-inner{display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:0.75rem}\
.grp-tag{font-weight:700;color:#f8fafc;font-size:0.92rem;display:flex;align-items:center;gap:0.5rem}\
.ch-badge{display:inline-flex;align-items:center;gap:0.4rem;font-weight:700;font-size:0.9rem}\
.dot{width:8px;height:8px;border-radius:50%;display:inline-block}\
.dot-fire{background:var(--orange);box-shadow:0 0 6px var(--orange)}\
.dot-ws{background:var(--cyan);box-shadow:0 0 6px var(--cyan)}\
.dot-fill{background:var(--purple);box-shadow:0 0 6px var(--purple)}\
.dot-pwm{background:var(--primary);box-shadow:0 0 6px var(--primary)}\
.dot-off{background:#475569}\
.ch-row.is-testing td{background:rgba(239,68,68,0.14)!important}\
.ch-row.is-testing:hover td{background:rgba(239,68,68,0.22)!important}\
.ch-row.is-testing .ch-badge{color:#f87171}\
.ch-row.is-testing .dot{background:#ef4444!important;box-shadow:0 0 8px #ef4444!important}\
.cell-na{display:none;color:var(--text-muted);font-size:1.1rem;text-align:center;line-height:34px;height:34px}\
.keep-case{text-transform:none}\
.unit-in{position:relative;display:inline-block}\
.unit-in input{padding-right:2rem!important;-moz-appearance:textfield}\
.unit-in input::-webkit-inner-spin-button,.unit-in input::-webkit-outer-spin-button{-webkit-appearance:none;margin:0}\
.unit-in span{position:absolute;right:9px;top:50%;transform:translateY(-50%);font-size:0.75rem;color:var(--text-muted);font-weight:600;pointer-events:none}\
.btn-ghost{background:transparent;color:inherit;border:1px solid currentColor;margin-left:0.4rem}\
.btn-ghost:hover{background:rgba(250,204,21,0.15);transform:none}\
.btn:disabled,.btn:disabled:hover{background:var(--surface-hdr);color:var(--text-muted);box-shadow:inset 0 0 0 1px var(--border);opacity:0.6;cursor:not-allowed;transform:none}\
.hint{font-size:0.72rem;color:var(--text-muted);font-weight:500;text-transform:none;letter-spacing:0}\
input:invalid{border-color:var(--red)}\
.ch-row.mode-off .cell-uni input,\
.ch-row.mode-off .cell-dmx input,\
.ch-row.mode-off .cell-px input,\
.ch-row.mode-off .cell-loss select,\
.ch-row.mode-off .cell-loss .c-select,\
.ch-row.mode-off .cell-test select,\
.ch-row.mode-off .cell-test .c-select,\
.ch-row.mode-pwm .cell-px input{display:none!important}\
.ch-row.mode-off .cell-uni .cell-na,\
.ch-row.mode-off .cell-dmx .cell-na,\
.ch-row.mode-off .cell-px .cell-na,\
.ch-row.mode-off .cell-loss .cell-na,\
.ch-row.mode-off .cell-test .cell-na,\
.ch-row.mode-pwm .cell-px .cell-na{display:block!important}\
</style></head><body><div class=\"container\">";

pub fn render_head_and_styles<W: core::fmt::Write>(w: &mut W) -> core::fmt::Result {
    w.write_str(HEAD_AND_STYLES)
}

pub fn render_topbar<W: core::fmt::Write>(w: &mut W, data: &PageData<'_>) -> core::fmt::Result {
    let test_display = if data.test_mode != 0 {
        "inline-flex"
    } else {
        "none"
    };

    write!(
        w,
        "<div class=\"topbar\">\
         <div class=\"brand\"><div class=\"brand-logo\"><svg viewBox=\"0 0 32 32\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.6\" stroke-linecap=\"round\" aria-hidden=\"true\"><path d=\"M16 29V20M16 20C16 14 7 15 7 7M16 20V7M16 20C16 14 25 15 25 7\"/><g fill=\"currentColor\" stroke=\"none\"><circle cx=\"7\" cy=\"5.5\" r=\"3\"/><circle cx=\"16\" cy=\"5.5\" r=\"3\"/><circle cx=\"25\" cy=\"5.5\" r=\"3\"/></g></svg></div>\
         <div><div class=\"brand-title\">hyd<span class=\"brand-ra\">ra</span><span class=\"brand-nano\">NANO</span></div>\
         <div class=\"brand-sub\">8-Channel sACN Output Controller</div></div></div>\
         <div class=\"topbar-pills\">\
         <span id=\"pill-test\" class=\"pill pill-yellow\" style=\"display:{test_display}\"><span class=\"pulse\"></span> test mode active \
         <button type=\"button\" class=\"btn btn-sm btn-ghost\" onclick=\"stopTests()\">Stop all</button></span>\
         <span id=\"pill-link\" class=\"pill pill-red\" style=\"display:none\">connection lost</span></div></div>"
    )
}

pub fn render_stats_grid<W: core::fmt::Write>(w: &mut W, data: &PageData<'_>) -> core::fmt::Result {
    let uptime = data.uptime_secs;
    let m = data.mac;

    write!(
        w,
        "<div class=\"grid-stats\">\
         <div class=\"stat-card\">\
         <div class=\"stat-item\"><div class=\"stat-label\">IP Address</div>\
         <div class=\"stat-val\">"
    )?;

    match data.ip {
        Some((ip, pfx)) => {
            let mode = if data.is_fallback {
                "<span class=\"pill pill-gray\" style=\"color:var(--text-muted);font-weight:600;font-size:0.7rem;padding:2px 7px\">fallback</span>"
            } else if data.dhcp_enabled {
                "<span class=\"pill pill-green keep-case\">DHCP</span>"
            } else {
                "<span class=\"pill pill-cyan\">static</span>"
            };
            write!(
                w,
                "{}.{}.{}.{}/{} {}",
                ip[0], ip[1], ip[2], ip[3], pfx, mode
            )?;
        }
        None => {
            w.write_str("None <span class=\"pill pill-red\">Disconnected</span>")?;
        }
    }

    write!(
        w,
        "</div></div>\
         <div class=\"stat-item\"><div class=\"stat-label\">MAC Address</div>\
         <div class=\"stat-val stat-mono\">{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}</div></div>\
         </div>\
         <div class=\"stat-card\">\
         <div class=\"stat-item\"><div class=\"stat-label\">Hostname</div>\
         <div class=\"stat-val\">{}</div></div>\
         <div class=\"stat-item\"><div class=\"stat-label\">Ports</div>\
         <div class=\"stat-val\" style=\"font-size:0.95rem;font-weight:600\">80 <span style=\"font-size:0.75rem;color:var(--text-muted)\">HTTP</span> &bull; 5568 <span style=\"font-size:0.75rem;color:var(--text-muted)\">sACN</span></div></div>\
         </div>\
         <div class=\"stat-card\">\
         <div class=\"stat-item\"><div class=\"stat-label\">System Uptime</div>\
         <div class=\"stat-val\" id=\"stat-uptime-val\">{}h {:02}m {:02}s</div></div>\
         <div class=\"stat-item\"><div class=\"stat-label\">Firmware Version</div>\
         <div class=\"stat-val stat-mono\" style=\"color:var(--text-muted);font-weight:600\">v{}</div></div>\
         </div>\
         </div>",
        m[0], m[1], m[2], m[3], m[4], m[5],
        data.hostname,
        uptime / 3600,
        (uptime / 60) % 60,
        uptime % 60,
        data.version
    )
}

const FOOTER_STYLE: &str = "padding:1rem 1.25rem;background:var(--surface-hdr);border-top:1px solid var(--border);border-bottom-left-radius:11px;border-bottom-right-radius:11px;display:flex;justify-content:flex-end";

pub fn render_network_card<W: core::fmt::Write>(
    w: &mut W,
    data: &PageData<'_>,
) -> core::fmt::Result {
    let sel_dhcp = if data.dhcp_enabled { " selected" } else { "" };
    let sel_static = if !data.dhcp_enabled { " selected" } else { "" };
    let s_ip = data.static_ip;
    let ip_label = if data.dhcp_enabled {
        "Fallback IP"
    } else {
        "Static IP"
    };

    write!(
        w,
        "<div class=\"card\"><div class=\"card-hdr\"><h2 class=\"card-title\"><span>🌐</span> Network Settings</h2></div>\
         <form id=\"net-form\" autocomplete=\"off\" method=\"POST\" action=\"/network\" data-def-host=\"{}\" data-def-ip=\"{}.{}.{}.{}\" data-def-prefix=\"{}\" onsubmit=\"_pending=false\">\
         <div class=\"card-body\">\
         <div class=\"net-form\">\
         <div class=\"f-grp\"><label>Hostname</label><input name=\"hostname\" type=\"text\" value=\"{}\" maxlength=\"32\" required pattern=\"[A-Za-z0-9][A-Za-z0-9\\-]*\" title=\"Letters, digits and hyphens only\" autocomplete=\"off\" spellcheck=\"false\"></div>\
         <div class=\"f-grp\"><label>IP Assignment</label><select name=\"dhcp\" id=\"net-dhcp-sel\">\
         <option value=\"true\"{}>DHCP (with fallback)</option>\
         <option value=\"false\"{}>Static IP</option></select></div>\
         <div class=\"f-grp\"><label id=\"net-ip-lbl\">{}</label><input name=\"ip\" id=\"net-ip\" type=\"text\" value=\"{}.{}.{}.{}\" inputmode=\"decimal\" pattern=\"[0-9]{{1,3}}(\\.[0-9]{{1,3}}){{3}}\" autocomplete=\"off\"></div>\
         <div class=\"f-grp\"><label>Subnet Prefix Length</label><input name=\"prefix\" id=\"net-prefix\" type=\"number\" min=\"1\" max=\"32\" value=\"{}\"></div>\
         </div></div>\
         <div style=\"{FOOTER_STYLE}\">\
         <button type=\"button\" id=\"net-reset\" class=\"btn btn-lg btn-ghost\" title=\"Load the factory hostname and DHCP into the fields (not saved until you press Save)\">Reset to defaults</button>\
         <button type=\"submit\" id=\"net-save\" class=\"btn btn-lg\" style=\"margin-left:0.6rem\" disabled>Save</button>\
         </div></form></div>",
        data.default_hostname,
        data.default_ip[0], data.default_ip[1], data.default_ip[2], data.default_ip[3],
        crate::config::NetConfig::default().static_prefix,
        data.hostname,
        sel_dhcp,
        sel_static,
        ip_label,
        s_ip[0], s_ip[1], s_ip[2], s_ip[3],
        data.static_prefix,
    )
}

pub fn render_channel_table_header<W: core::fmt::Write>(
    w: &mut W,
    data: &PageData<'_>,
) -> core::fmt::Result {
    let (pill_cls, pill_txt, pill_op, unsaved) = if data.unsaved {
        (
            "pill-yellow",
            "<span class=\"pulse\" style=\"background:#facc15\"></span> pending changes",
            "1",
            "1",
        )
    } else {
        ("pill-yellow", "", "0", "0")
    };

    write!(
        w,
        "<div class=\"card\">\
         <div class=\"card-hdr\">\
         <div style=\"display:flex;align-items:center;gap:0.75rem;flex-wrap:wrap\">\
         <h2 class=\"card-title\"><span>🎛️</span> Channel Configuration</h2>\
         <span id=\"save-pill\" data-unsaved=\"{unsaved}\" class=\"pill {pill_cls}\" style=\"font-size:0.72rem;padding:3px 10px;opacity:{pill_op};transition:opacity 0.25s\">{pill_txt}</span>\
         </div>\
         <div style=\"display:flex;align-items:center;gap:1.25rem;flex-wrap:wrap\">\
         <div style=\"display:flex;align-items:center;gap:0.6rem\" title=\"Time without sACN data before the Loss Behavior takes effect (all channels)\">\
         <label style=\"font-size:0.75rem;font-weight:700;color:var(--text-muted);text-transform:uppercase\">Loss timeout</label>\
         <span class=\"unit-in\"><input type=\"number\" id=\"loss-timeout-input\" autocomplete=\"off\" name=\"g_loss\" min=\"100\" max=\"60000\" step=\"100\" value=\"{}\" style=\"width:90px;padding:4px 8px;font-size:0.85rem\"><span>ms</span></span></div>\
         <div id=\"pwm-freq-wrap\" style=\"display:flex;align-items:center;gap:0.6rem\">\
         <label style=\"font-size:0.75rem;font-weight:700;color:var(--text-muted);text-transform:uppercase\">PWM Frequency</label>\
         <span class=\"unit-in\"><input type=\"number\" id=\"pwm-freq-input\" autocomplete=\"off\" name=\"g_pwm\" min=\"100\" max=\"20000\" value=\"{}\" style=\"width:90px;padding:4px 8px;font-size:0.85rem\"><span>Hz</span></span>\
         </div></div></div>\
         <form id=\"outputs-form\" autocomplete=\"off\" method=\"POST\" action=\"/outputs/save\" onsubmit=\"return saveOutputs(event);\">\
         <table class=\"table table-fixed\"><thead><tr>\
         <th style=\"width:100px\">Channel</th>\
         <th style=\"width:170px\">Mode</th>\
         <th style=\"width:80px;text-align:center\">Universe</th>\
         <th style=\"width:85px;text-align:center\">DMX Start</th>\
         <th style=\"width:70px;text-align:center\">Pixels</th>\
         <th style=\"width:120px;text-align:center\">DMX End</th>\
         <th style=\"width:105px\">Loss</th>\
         <th style=\"width:150px\">Test Pattern</th>\
         </tr></thead><tbody>",
        data.loss_timeout_ms,
        data.pwm_freq_hz
    )
}

pub fn render_channel_table_footer<W: core::fmt::Write>(w: &mut W) -> core::fmt::Result {
    write!(
        w,
        "</tbody></table>\
         <div style=\"{FOOTER_STYLE}\">\
         <button type=\"button\" id=\"revert-outputs-btn\" class=\"btn btn-lg btn-ghost\" title=\"Discard all pending changes and restore the saved settings\" onclick=\"revertOutputs()\" disabled>Revert pending changes</button>\
         <button type=\"submit\" id=\"save-outputs-btn\" class=\"btn btn-lg\" style=\"margin-left:0.6rem\">Save</button>\
         </div></form></div>"
    )
}

/// Highest valid first DMX slot for a mode (3-slot modes must fit in the universe).
fn max_dmx_start(mode_str: &str) -> u16 {
    match mode_str {
        "ws2812fill" | "fire" => 510,
        _ => 512,
    }
}

pub fn render_channel_row<W: core::fmt::Write>(
    w: &mut W,
    ch_idx: usize,
    ch: &ChannelView,
) -> core::fmt::Result {
    let ch_num = ch_idx + 1;
    let (uni, dmx, px) = match ch.mode {
        ChannelMode::Ws2812 {
            universe,
            start_channel,
            pixel_count,
            ..
        } => (universe, start_channel, pixel_count),
        ChannelMode::Ws2812Fill {
            universe,
            start_channel,
            pixel_count,
        } => (universe, start_channel, pixel_count),
        ChannelMode::Fire {
            pixel_count,
            control,
        } => (control.universe, control.channel, pixel_count),
        ChannelMode::Pwm {
            universe,
            dmx_channel,
        } => (universe, dmx_channel, 30),
        ChannelMode::Off => (1, 1, 30),
    };

    let (dot_class, mode_str) = match ch.mode {
        ChannelMode::Off => ("dot-off", "off"),
        ChannelMode::Ws2812 { .. } => ("dot-ws", "ws2812"),
        ChannelMode::Ws2812Fill { .. } => ("dot-fill", "ws2812fill"),
        ChannelMode::Fire { .. } => ("dot-fire", "fire"),
        ChannelMode::Pwm { .. } => ("dot-pwm", "pwm"),
    };
    let dmx_max = max_dmx_start(mode_str);

    let is_off = matches!(ch.mode, ChannelMode::Off);
    let is_ws = matches!(ch.mode, ChannelMode::Ws2812 { .. });
    let is_fill = matches!(ch.mode, ChannelMode::Ws2812Fill { .. });
    let is_fire = matches!(ch.mode, ChannelMode::Fire { .. });
    let is_pwm = matches!(ch.mode, ChannelMode::Pwm { .. });

    let is_blackout = matches!(ch.loss_behavior, LossBehavior::Blackout);
    let is_hold = matches!(ch.loss_behavior, LossBehavior::Hold);
    let is_fade = matches!(ch.loss_behavior, LossBehavior::Fade { .. });

    let is_t0 = ch.test_mode == 0;
    let is_t1 = ch.test_mode == 1;
    let is_t2 = ch.test_mode == 2;
    let is_t3 = ch.test_mode == 3;
    let is_t4 = ch.test_mode == 4;
    let is_t5 = ch.test_mode == 5;
    let test_cls = if is_t0 { "" } else { " is-testing" };

    write!(
        w,
        "<tr class=\"ch-row mode-{mode_str}{test_cls}\" data-ch=\"{ch_num}\">\
         <td><span class=\"ch-badge\"><span class=\"dot {dot_class}\"></span> Channel {ch_num}</span></td>\
         <td><select name=\"m{ch_idx}\">\
         <option value=\"off\"{}>Off</option>\
         <option value=\"ws2812\"{}>WS2812B RGB</option>\
         <option value=\"ws2812fill\"{}>WS2812B Fill</option>\
         <option value=\"fire\"{}>Fire Effect</option>\
         <option value=\"pwm\"{}>PWM Dimmer</option>\
         </select></td>",
        if is_off { " selected" } else { "" },
        if is_ws { " selected" } else { "" },
        if is_fill { " selected" } else { "" },
        if is_fire { " selected" } else { "" },
        if is_pwm { " selected" } else { "" }
    )?;

    write!(
        w,
        "<td class=\"cell-uni\"><input type=\"number\" name=\"u{ch_idx}\" min=\"1\" max=\"63999\" value=\"{uni}\"><span class=\"cell-na\">&mdash;</span></td>\
         <td class=\"cell-dmx\"><input type=\"number\" name=\"d{ch_idx}\" min=\"1\" max=\"{dmx_max}\" value=\"{dmx}\"><span class=\"cell-na\">&mdash;</span></td>\
         <td class=\"cell-px\"><input type=\"number\" name=\"p{ch_idx}\" min=\"1\" max=\"1000\" value=\"{px}\"><span class=\"cell-na\">&mdash;</span></td>"
    )?;

    // DMX End calculation
    let (end_u, end_ch, ch_count) = match ch.mode {
        ChannelMode::Off => (0, 0, 0),
        ChannelMode::Pwm {
            universe,
            dmx_channel,
        } => (universe, dmx_channel, 1),
        ChannelMode::Fire { control, .. } => (control.universe, (control.channel + 2).min(512), 3),
        ChannelMode::Ws2812Fill {
            universe,
            start_channel,
            ..
        } => (universe, (start_channel + 2).min(512), 3),
        ChannelMode::Ws2812 {
            universe,
            start_channel,
            pixel_count,
        } => {
            let total = (pixel_count as usize) * 3;
            let mut cur_u = universe;
            let mut cur_c = (start_channel as usize).clamp(1, 512);
            let mut rem = pixel_count as usize;
            let mut res = (universe, start_channel, total);
            while rem > 0 {
                let avail = 512 + 1 - cur_c;
                let fit = avail / 3;
                if fit >= rem {
                    let end_c = (cur_c + rem * 3 - 1) as u16;
                    res = (cur_u, end_c, total);
                    break;
                }
                rem -= fit;
                cur_u = cur_u.saturating_add(1);
                cur_c = 1;
            }
            res
        }
    };

    if ch_count > 0 {
        write!(
            w,
            "<td class=\"cell-end\" style=\"text-align:center\"><span class=\"cell-end-val stat-mono\" style=\"font-size:0.82rem\">U{end_u}:{end_ch} <span style=\"font-size:0.72rem;color:var(--text-muted)\">({ch_count}ch)</span></span></td>"
        )?;
    } else {
        w.write_str(
            "<td class=\"cell-end\" style=\"text-align:center\"><span class=\"cell-end-val stat-mono\" style=\"font-size:0.82rem\"><span class=\"cell-na\" style=\"display:inline\">&mdash;</span></span></td>"
        )?;
    }

    write!(
        w,
        "<td class=\"cell-loss\"><select name=\"l{ch_idx}\">\
         <option value=\"blackout\"{}>Blackout</option>\
         <option value=\"hold\"{}>Hold</option>\
         <option value=\"fade\"{}>Fade</option>\
         </select><span class=\"cell-na\">&mdash;</span></td>\
         <td class=\"cell-test\"><select name=\"t{ch_idx}\" class=\"test-sel\" data-ch=\"{ch_num}\">",
        if is_blackout { " selected" } else { "" },
        if is_hold { " selected" } else { "" },
        if is_fade { " selected" } else { "" }
    )?;

    write!(
        w,
        "<option value=\"0\"{}>Live (sACN)</option>",
        if is_t0 { " selected" } else { "" }
    )?;

    match ch.mode {
        ChannelMode::Ws2812 { .. } | ChannelMode::Ws2812Fill { .. } | ChannelMode::Fire { .. } => {
            write!(
                w,
                "<option value=\"3\"{}>Rainbow Wave</option>\
                 <option value=\"2\"{}>Color Cycle</option>\
                 <option value=\"5\"{}>Pixel Tracer</option>\
                 <option value=\"1\"{}>100% White</option>\
                 <option value=\"4\"{}>Breathe</option>",
                if is_t3 { " selected" } else { "" },
                if is_t2 { " selected" } else { "" },
                if is_t5 { " selected" } else { "" },
                if is_t1 { " selected" } else { "" },
                if is_t4 { " selected" } else { "" },
            )?;
        }
        ChannelMode::Pwm { .. } => {
            write!(
                w,
                "<option value=\"1\"{}>100% Full On</option>\
                 <option value=\"4\"{}>Breathe</option>",
                if is_t1 { " selected" } else { "" },
                if is_t4 { " selected" } else { "" },
            )?;
        }
        ChannelMode::Off => {}
    }

    w.write_str("</select><span class=\"cell-na\">&mdash;</span></td></tr>")
}

const EMPTY_UNIVERSES_ROW: &str =
    "<tr><td colspan=\"5\" style=\"color:var(--text-muted);text-align:center;padding:1.5rem\">\
    Listening on UDP port 5568 &mdash; send sACN packets to see live stream data</td></tr>";

pub fn render_universes_header<W: core::fmt::Write>(w: &mut W) -> core::fmt::Result {
    w.write_str(
        "<div class=\"card\"><div class=\"card-hdr\">\
         <h2 class=\"card-title\"><span>📡</span> sACN Universes</h2></div>\
         <table class=\"table\"><thead><tr>\
         <th>Universe</th><th>Used by</th><th>Last Sequence</th><th>Packets Received</th><th>Live Status</th>\
         </tr></thead><tbody id=\"uni-table-body\">",
    )
}

/// One `<tr>` for the universes table. Renders into small buffers, so callers
/// can stream rows one at a time.
pub fn render_universe_row<W: core::fmt::Write>(w: &mut W, u: &UniverseView) -> core::fmt::Result {
    let used = u.used_by != 0;
    let pill = if u.terminated {
        "<span class=\"pill pill-yellow\">terminated</span>"
    } else if u.active {
        "<span class=\"pill pill-green\"><span class=\"pulse\"></span> active</span>"
    } else if used && u.packets == 0 {
        "<span class=\"pill pill-red\">no signal</span>"
    } else if used {
        "<span class=\"pill pill-red\">signal lost</span>"
    } else {
        "<span class=\"pill pill-gray\"><span class=\"dot dot-off\"></span> inactive</span>"
    };
    write!(
        w,
        "<tr><td><strong>Universe {}</strong></td><td>",
        u.universe
    )?;
    if used {
        let mut first = true;
        for bit in 0..CHANNEL_COUNT {
            if u.used_by & (1 << bit) != 0 {
                write!(w, "{}Ch {}", if first { "" } else { ", " }, bit + 1)?;
                first = false;
            }
        }
    } else {
        w.write_str("<span style=\"color:var(--text-muted)\">&mdash;</span>")?;
    }
    w.write_str("</td><td>")?;
    if u.packets == 0 {
        w.write_str("&mdash;")?;
    } else {
        write!(w, "{}", u.sequence)?;
    }
    write!(w, "</td><td>{}</td><td>{}</td></tr>", u.packets, pill)
}

pub fn render_universes_empty_row<W: core::fmt::Write>(w: &mut W) -> core::fmt::Result {
    w.write_str(EMPTY_UNIVERSES_ROW)
}

pub fn render_universes_footer<W: core::fmt::Write>(w: &mut W) -> core::fmt::Result {
    w.write_str("</tbody></table></div>")
}

/// Whole universes card in one go (host preview / tests). The firmware streams
/// the header, rows and footer separately to stay within its chunk buffer.
pub fn render_universes_card<W: core::fmt::Write>(
    w: &mut W,
    universes: &[UniverseView],
) -> core::fmt::Result {
    render_universes_header(w)?;
    if universes.is_empty() {
        render_universes_empty_row(w)?;
    }
    for u in universes {
        render_universe_row(w, u)?;
    }
    render_universes_footer(w)
}

pub static SCRIPT: &str = "<script>\
function enc(v){return encodeURIComponent(v);}\
function post(url,body){return fetch(url,{method:'POST',headers:{'Content-Type':'application/x-www-form-urlencoded'},body:body});}\
function chk(r){if(!r.ok)syncErr();return r;}\
function initCustomSelect(sel){\
if(!sel)return;\
if(sel._c){sel._c();return;}\
sel.style.display='none';\
var wrap=document.createElement('div');wrap.className='c-select';\
var btn=document.createElement('button');btn.type='button';btn.className='c-select-btn';\
var lbl=document.createElement('span');btn.appendChild(lbl);\
var menu=document.createElement('div');menu.className='c-select-menu';\
wrap.appendChild(btn);wrap.appendChild(menu);\
sel.parentNode.insertBefore(wrap,sel.nextSibling);\
var sync=function(){\
menu.innerHTML='';\
var cur=sel.value,curTxt='';\
for(var i=0;i<sel.options.length;i++){\
var opt=sel.options[i];\
if(opt.value===cur||(!curTxt&&i===0)){curTxt=opt.text;}\
var itm=document.createElement('div');\
itm.className='c-select-item'+(opt.value===cur?' selected':'');\
itm.textContent=opt.text;\
(function(val){\
itm.addEventListener('click',function(e){\
e.stopPropagation();sel.value=val;\
wrap.classList.remove('open');\
sel.dispatchEvent(new Event('change',{bubbles:true}));\
sync();\
});\
})(opt.value);\
menu.appendChild(itm);\
}\
lbl.textContent=curTxt||(sel.options[0]?sel.options[0].text:'');\
};\
btn.addEventListener('click',function(e){\
e.stopPropagation();var isOp=wrap.classList.contains('open');\
document.querySelectorAll('.c-select.open').forEach(function(c){c.classList.remove('open');});\
if(!isOp){sync();wrap.classList.add('open');}\
});\
btn.addEventListener('keydown',function(e){if(e.key==='Escape')wrap.classList.remove('open');});\
sel._c=sync;\
sync();\
}\
function rowMode(tr){var ms=tr.querySelector('select[name^=m]');return ms?ms.value:'off';}\
function numVal(inp,def){var v=parseInt(inp&&inp.value,10);return isNaN(v)?def:v;}\
function clampInput(t){var v=parseInt(t.value,10);var mn=+t.min,mx=+t.max;if(isNaN(v))v=mn;t.value=Math.min(Math.max(v,mn),mx);}\
function limits(tr){\
var m=rowMode(tr),ds=tr.querySelector('input[name^=d]');\
if(!ds)return;\
ds.min=1;\
ds.max=(m==='ws2812fill'||m==='fire')?510:512;\
}\
function rowClasses(tr){\
var m=rowMode(tr);\
var ds=tr.querySelector('input[name^=d]');\
var ts=tr.querySelector('select[name^=t]');\
var c='ch-row mode-'+m;\
if(ts&&ts.value!=='0')c+=' is-testing';\
tr.className=c;\
}\
function getTestOpts(m){\
var n='<option value=\"0\">Live (sACN)</option>';\
if(m==='ws2812'||m==='ws2812fill'||m==='fire'){\
return n+'<option value=\"3\">Rainbow Wave</option><option value=\"2\">Color Cycle</option><option value=\"5\">Pixel Tracer</option><option value=\"1\">100% White</option><option value=\"4\">Breathe</option>';\
}else if(m==='pwm'){\
return n+'<option value=\"1\">100% Full On</option><option value=\"4\">Breathe</option>';\
}\
return n;\
}\
var DIM='<span style=\"font-size:0.72rem;color:var(--text-muted)\">';\
function calcDmxEnd(m,u,d,p){\
if(m==='off')return'<span class=\"cell-na\" style=\"display:inline\">&mdash;</span>';\
if(m==='pwm')return'U'+u+':'+d+' '+DIM+'(1ch)</span>';\
if(m==='fire'||m==='ws2812fill'){return'U'+u+':'+Math.min(d+2,512)+' '+DIM+'(3ch)</span>';}\
if(m==='ws2812'){\
var rem=p,curU=u,curCh=Math.min(Math.max(d,1),512);\
while(rem>0){\
var fit=Math.floor((513-curCh)/3);\
if(fit>=rem){return'U'+curU+':'+(curCh+rem*3-1)+' '+DIM+'('+(p*3)+'ch)</span>';}\
rem-=fit;curU++;curCh=1;\
}\
}\
return'<span class=\"cell-na\" style=\"display:inline\">&mdash;</span>';\
}\
function updDmxEnd(tr){\
if(!tr)return;\
var endEl=tr.querySelector('.cell-end-val');\
if(!endEl)return;\
endEl.innerHTML=calcDmxEnd(rowMode(tr),numVal(tr.querySelector('input[name^=u]'),1),numVal(tr.querySelector('input[name^=d]'),1),Math.max(1,numVal(tr.querySelector('input[name^=p]'),30)));\
}\
var _deb={};\
function debounce(key,fn){\
if(_deb[key])clearTimeout(_deb[key].t);\
_deb[key]={fn:fn,t:setTimeout(function(){delete _deb[key];fn();},350)};\
}\
function cancelDebounce(key){if(_deb[key]){clearTimeout(_deb[key].t);delete _deb[key];}}\
function flushDebounced(){\
var ps=[];\
for(var k in _deb){clearTimeout(_deb[k].t);ps.push(_deb[k].fn());}\
_deb={};\
return ps;\
}\
var _saving=false,_lastEdit=0,_lastTestEdit=0;\
var _pill=document.getElementById('save-pill');\
var _pending=!!(_pill&&_pill.getAttribute('data-unsaved')==='1');\
function showPill(cls,html){\
if(!_pill)return;\
_pill.className='pill '+cls;_pill.innerHTML=html;_pill.style.opacity='1';\
}\
function hidePill(){if(_pill)_pill.style.opacity='0';}\
function updSaveBtn(){\
var b=document.getElementById('save-outputs-btn');\
if(b)b.disabled=_saving||!_pending;\
var r=document.getElementById('revert-outputs-btn');\
if(r)r.disabled=_saving||!_pending;\
}\
function markPending(p){\
_pending=p;\
updSaveBtn();\
if(p){\
_lastEdit=Date.now();\
showPill('pill-yellow','<span class=\"pulse\" style=\"background:#facc15\"></span> pending changes');\
}\
}\
function syncErr(){showPill('pill-red','Device unreachable - change not applied');}\
async function revertOutputs(){\
if(!_pending||_saving)return;\
if(!confirm('Revert all pending changes and restore the saved settings?'))return;\
_saving=true;updSaveBtn();\
try{\
_deb={};\
var r=await fetch('/api/outputs/revert',{method:'POST'});\
if(r.ok){_pending=false;location.replace('/');return;}\
showPill('pill-red','revert failed ('+r.status+')');\
}catch(err){\
showPill('pill-red','revert failed - network error');\
}\
_saving=false;updSaveBtn();\
}\
async function saveOutputs(e){\
if(e&&e.preventDefault)e.preventDefault();\
var btn=document.getElementById('save-outputs-btn');\
_saving=true;\
if(btn)btn.textContent='Saving...';\
updSaveBtn();\
showPill('pill-cyan','<span class=\"pulse\" style=\"background:#38bdf8\"></span> Saving...');\
try{\
await Promise.all(flushDebounced());\
var r=await fetch('/api/outputs/save',{method:'POST'});\
if(r.ok){\
_pending=false;\
updSaveBtn();\
showPill('pill-green','<span class=\"pulse\" style=\"background:#34d399\"></span> saved');\
setTimeout(function(){if(!_pending)hidePill();},3000);\
}else{\
showPill('pill-red',r.status===500?'save failed':'save failed ('+r.status+')');\
}\
}catch(err){\
showPill('pill-red','save failed - network error');\
}finally{\
_saving=false;\
if(btn)btn.textContent='Save';\
updSaveBtn();\
}\
return false;\
}\
function syncChannel(tr){\
if(!tr)return Promise.resolve();\
var ch=tr.getAttribute('data-ch');\
if(!ch)return Promise.resolve();\
var ls=tr.querySelector('select[name^=l]');\
var body='ch='+enc(ch)+'&mode='+enc(rowMode(tr))\
+'&universe='+enc(numVal(tr.querySelector('input[name^=u]'),1))\
+'&dmx_channel='+enc(numVal(tr.querySelector('input[name^=d]'),1))\
+'&pixel_count='+enc(numVal(tr.querySelector('input[name^=p]'),30))\
+'&loss='+enc(ls?ls.value:'blackout');\
return post('/api/channel',body).then(chk).catch(syncErr);\
}\
function syncTest(tr){\
if(!tr)return;\
var ts=tr.querySelector('select[name^=t]');\
if(!ts)return;\
_lastTestEdit=Date.now();\
var ch=ts.getAttribute('data-ch');\
var px=tr.querySelector('input[name^=p]');\
post('/api/test/channel','ch='+enc(ch)+'&test='+enc(ts.value)+'&ch_mode='+enc(rowMode(tr))+'&pixels='+enc(px?px.value:'')).then(chk).catch(syncErr);\
}\
function setTestPill(on){\
var p=document.getElementById('pill-test');\
if(p)p.style.display=on?'inline-flex':'none';\
}\
function stopTests(){\
post('/api/test/all','mode=0').then(chk).then(function(){\
document.querySelectorAll('select.test-sel').forEach(function(s){\
s.value='0';if(s._c)s._c();\
var tr=s.closest('tr');if(tr)rowClasses(tr);\
});\
setTestPill(false);\
}).catch(syncErr);\
}\
function updRow(s){\
var tr=s.closest('tr');if(!tr)return;\
var m=s.value;\
var ds=tr.querySelector('input[name^=d]');\
limits(tr);\
if(ds){\
if(numVal(ds,0)<1)ds.value=1;\
clampInput(ds);\
}\
rowClasses(tr);\
var d=tr.querySelector('.dot');\
if(d){d.className='dot '+(m==='off'?'dot-off':m==='pwm'?'dot-pwm':m==='fire'?'dot-fire':m==='ws2812fill'?'dot-fill':'dot-ws');}\
updDmxEnd(tr);\
var ts=tr.querySelector('select[name^=t]');\
if(ts){\
var curT=ts.value;\
ts.innerHTML=getTestOpts(m);\
var valid=false;\
for(var i=0;i<ts.options.length;i++){if(ts.options[i].value===curT){valid=true;break;}}\
ts.value=valid?curT:'0';\
if(ts._c)ts._c();\
rowClasses(tr);\
if(curT!=='0'&&(!valid||m==='off')){\
post('/api/test/channel','ch='+enc(ts.getAttribute('data-ch'))+'&test=0').catch(syncErr);\
}else if(ts.value!=='0'){\
syncTest(tr);\
}\
}\
cancelDebounce(tr.getAttribute('data-ch'));\
syncChannel(tr);\
markPending(true);\
}\
var _netInit=null;\
function netVals(f){return [f.hostname.value,f.dhcp.value,f.ip.value,f.prefix.value].join('|');}\
function updNet(){\
var f=document.getElementById('net-form');if(!f)return;\
if(_netInit===null)_netInit=netVals(f);\
var b=document.getElementById('net-save');if(b)b.disabled=netVals(f)===_netInit;\
var l=document.getElementById('net-ip-lbl');if(l)l.textContent=f.dhcp.value==='true'?'Fallback IP':'Static IP';\
}\
(function(){\
var f=document.getElementById('net-form');if(!f)return;\
_netInit=netVals(f);\
var r=document.getElementById('net-reset');\
if(r)r.addEventListener('click',function(){\
f.hostname.value=f.getAttribute('data-def-host');\
f.dhcp.value='true';if(f.dhcp._c)f.dhcp._c();\
f.ip.value=f.getAttribute('data-def-ip');\
f.prefix.value=f.getAttribute('data-def-prefix');\
updNet();\
});\
})();\
function handle(t,fin){\
if(!t||!t.name&&!t.id)return;\
if(t.closest&&t.closest('#net-form')){updNet();return;}\
if(t.name==='g_pwm'||t.name==='g_loss'){\
if(fin)clampInput(t);\
var isPwm=t.name==='g_pwm';\
var send=function(){return post(isPwm?'/api/pwm_freq':'/api/loss',(isPwm?'freq=':'timeout=')+enc(t.value)).then(chk).catch(syncErr);};\
if(fin){cancelDebounce(t.name);send();}else{debounce(t.name,send);}\
markPending(true);\
return;\
}\
var tr=t.closest&&t.closest('tr.ch-row');\
if(!tr||!t.name)return;\
var k=t.name.charAt(0);\
if(k==='m'){if(fin)updRow(t);}\
else if(k==='l'){if(fin){syncChannel(tr);markPending(true);}}\
else if(k==='t'){if(fin){rowClasses(tr);syncTest(tr);}}\
else if(k==='u'||k==='d'||k==='p'){\
var ch=tr.getAttribute('data-ch');\
if(fin){clampInput(t);cancelDebounce(ch);}\
if(k==='d')rowClasses(tr);\
updDmxEnd(tr);\
if(fin){syncChannel(tr);}else{debounce(ch,function(){return syncChannel(tr);});}\
markPending(true);\
var ts=tr.querySelector('select[name^=t]');\
if(k==='p'&&ts&&ts.value!=='0')syncTest(tr);\
}\
}\
document.addEventListener('change',function(e){handle(e.target,true);});\
document.addEventListener('input',function(e){if(e.target&&e.target.tagName==='INPUT')handle(e.target,false);});\
document.addEventListener('click',function(){\
document.querySelectorAll('.c-select.open').forEach(function(c){c.classList.remove('open');});\
});\
window.addEventListener('beforeunload',function(e){if(_pending){e.preventDefault();e.returnValue='';}});\
function outs(m){var a=[];for(var i=0;i<8;i++){if(m&(1<<i))a.push('Ch '+(i+1));}return a.join(', ');}\
function uniRow(u){\
var used=u.used_by!==0;\
var pill=u.terminated?'<span class=\"pill pill-yellow\">terminated</span>':u.active?'<span class=\"pill pill-green\"><span class=\"pulse\"></span> active</span>':(used&&u.packets===0)?'<span class=\"pill pill-red\">no signal</span>':used?'<span class=\"pill pill-red\">signal lost</span>':'<span class=\"pill pill-gray\"><span class=\"dot dot-off\"></span> inactive</span>';\
return'<tr><td><strong>Universe '+u.universe+'</strong></td><td>'+(used?outs(u.used_by):'<span style=\"color:var(--text-muted)\">&mdash;</span>')+'</td><td>'+(u.packets===0?'&mdash;':u.sequence)+'</td><td>'+u.packets+'</td><td>'+pill+'</td></tr>';\
}\
var _polling=false,_fails=0,_lastUp=-1;\
function pollStatus(){\
if(_polling)return;\
_polling=true;\
fetch('/api/status').then(function(r){if(!r.ok)throw new Error('http');return r.json();}).then(function(d){\
_fails=0;\
if(typeof d.uptime==='number'){\
if(_lastUp>=0&&d.uptime+2<_lastUp){location.reload();return;}\
_lastUp=d.uptime;\
var up=document.getElementById('stat-uptime-val');\
if(up){var h=Math.floor(d.uptime/3600),m=Math.floor((d.uptime%3600)/60),s=d.uptime%60;\
up.textContent=h+'h '+(m<10?'0':'')+m+'m '+(s<10?'0':'')+s+'s';}\
}\
var pl=document.getElementById('pill-link');if(pl)pl.style.display='none';\
if(Array.isArray(d.tests)){\
var any=false;\
for(var i=0;i<d.tests.length;i++){\
if(d.tests[i]!==0)any=true;\
if(Date.now()-_lastTestEdit<2500)continue;\
var tr=document.querySelector('tr[data-ch=\"'+(i+1)+'\"]');\
var ts=tr&&tr.querySelector('select.test-sel');\
if(ts&&ts.value!==String(d.tests[i])){\
ts.value=String(d.tests[i]);\
if(ts.value===String(d.tests[i])){if(ts._c)ts._c();rowClasses(tr);}\
}\
}\
setTestPill(any);\
}\
if(d.unsaved&&!_pending&&!_saving){_pending=true;updSaveBtn();showPill('pill-yellow','<span class=\"pulse\" style=\"background:#facc15\"></span> pending changes');}\
else if(!d.unsaved&&_pending&&!_saving&&Date.now()-_lastEdit>4000){_pending=false;updSaveBtn();hidePill();}\
var tb=document.getElementById('uni-table-body');\
if(tb&&Array.isArray(d.universes)){\
if(d.universes.length===0){\
tb.innerHTML='<tr><td colspan=\"5\" style=\"color:var(--text-muted);text-align:center;padding:1.5rem\">Listening on UDP port 5568 &mdash; send sACN packets to see live stream data</td></tr>';\
}else{\
var h2='';\
for(var j=0;j<d.universes.length;j++){h2+=uniRow(d.universes[j]);}\
tb.innerHTML=h2;\
}\
}\
}).catch(function(){\
_fails++;\
if(_fails>=3){var p=document.getElementById('pill-link');if(p)p.style.display='inline-flex';}\
}).finally(function(){_polling=false;});\
}\
setInterval(pollStatus,1500);\
document.querySelectorAll('select').forEach(initCustomSelect);\
document.querySelectorAll('tr.ch-row').forEach(function(tr){limits(tr);updDmxEnd(tr);});\
updNet();\
updSaveBtn();\
</script></div></body></html>";

pub fn render_script<W: core::fmt::Write>(w: &mut W) -> core::fmt::Result {
    w.write_str(SCRIPT)
}

/// Opening part of `/api/status`; follow with one [`render_status_json_universe`]
/// per universe and then [`render_status_json_tail`].
pub fn render_status_json_head<W: core::fmt::Write>(
    w: &mut W,
    uptime_secs: u64,
    link_up: bool,
    has_ip: bool,
    unsaved: bool,
    tests: &[u8; CHANNEL_COUNT],
) -> core::fmt::Result {
    write!(
        w,
        "{{\"uptime\":{},\"link\":{},\"ip\":{},\"unsaved\":{},\"tests\":[",
        uptime_secs, link_up, has_ip, unsaved
    )?;
    for (i, t) in tests.iter().enumerate() {
        if i > 0 {
            w.write_char(',')?;
        }
        write!(w, "{}", t)?;
    }
    w.write_str("],\"universes\":[")
}

pub fn render_status_json_universe<W: core::fmt::Write>(
    w: &mut W,
    first: bool,
    u: &UniverseView,
) -> core::fmt::Result {
    write!(
        w,
        "{}{{\"universe\":{},\"sequence\":{},\"packets\":{},\"active\":{},\"terminated\":{},\"used_by\":{}}}",
        if first { "" } else { "," },
        u.universe,
        u.sequence,
        u.packets,
        u.active,
        u.terminated,
        u.used_by
    )
}

pub fn render_status_json_tail<W: core::fmt::Write>(w: &mut W) -> core::fmt::Result {
    w.write_str("]}")
}

pub fn render_status_json<W: core::fmt::Write>(
    w: &mut W,
    uptime_secs: u64,
    link_up: bool,
    has_ip: bool,
    unsaved: bool,
    tests: &[u8; CHANNEL_COUNT],
    universes: &[UniverseView],
) -> core::fmt::Result {
    render_status_json_head(w, uptime_secs, link_up, has_ip, unsaved, tests)?;
    for (i, u) in universes.iter().enumerate() {
        render_status_json_universe(w, i == 0, u)?;
    }
    render_status_json_tail(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_channel() -> ChannelView {
        ChannelView {
            mode: ChannelMode::Ws2812 {
                universe: 63999,
                start_channel: 512,
                pixel_count: 1000,
            },
            loss_behavior: LossBehavior::Fade { ms: 1000 },
            loss_timeout_ms: 2500,
            test_mode: 5,
        }
    }

    #[test]
    fn channel_row_fits_firmware_chunk_buffer() {
        let mut s = String::new();
        render_channel_row(&mut s, 7, &sample_channel()).unwrap();
        // app/src/web.rs renders each row into a heapless::String<2048>.
        assert!(s.len() <= 2048, "row is {} bytes", s.len());
    }
}
