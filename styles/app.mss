/* ARDOR Edge Air Ultra — тёмная тема */

:root {
    --bg: #0d0f13;
    --bg-2: #12151b;
    --surface: #171a21;
    --surface-2: #1e222b;
    --surface-3: #262b36;
    --border: #2a2f3b;
    --border-strong: #3a4150;
    --accent: #ff3b4e;
    --accent-hover: #ff5a6b;
    --accent-soft: rgba(255, 59, 78, 0.14);
    --accent-line: rgba(255, 59, 78, 0.55);
    --text: #eceff4;
    --text-2: #b8bfcc;
    --muted: #7d8595;
    --ok: #34d399;
    --warn: #fbbf24;
    --bad: #f87171;
    --info: #60a5fa;
    --radius: 14px;

    --popup-background: #1e222b;
    --popup-color: #eceff4;
    --popup-border: #3a4150;
    --popup-accent: #ff3b4e;
    --popup-hover-background: #2a303c;
    --popup-hover-color: #ffffff;
    --popup-selected-background: rgba(255, 59, 78, 0.18);
    --popup-selected-color: #ffffff;
}

.grow { flex-grow: 1; }

Text { color: var(--text); font-size: 14px; }
Icon { color: var(--text-2); icon-size: 20px; }

.root { background: var(--bg); }

/* ---------- шапка ---------- */

.header {
    background: var(--bg-2);
    border-bottom-width: 1px;
    border-bottom-color: var(--border);
    padding: 14px 22px;
}

.brand-badge {
    width: 42px;
    height: 42px;
    border-radius: 12px;
    background: linear-gradient(135deg, #ff3b4e, #9b1c3a);
    box-shadow: 0 2px 10px rgba(255, 59, 78, 0.25);
}
.brand-icon { color: #ffffff; icon-size: 24px; }
.brand-kicker { color: var(--accent); font-size: 11px; font-weight: bold; letter-spacing: 2px; }
.brand-title { color: var(--text); font-size: 19px; font-weight: bold; }

.pill {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 18px;
    padding: 7px 14px;
}
.pill-text { color: var(--text-2); font-size: 13px; }

.dot { width: 9px; height: 9px; border-radius: 5px; background: var(--muted); }
.dot-ok { background: var(--ok); box-shadow: 0 0 8px rgba(52, 211, 153, 0.8); }
.dot-warn { background: var(--warn); box-shadow: 0 0 8px rgba(251, 191, 36, 0.7); }
.dot-bad { background: var(--bad); box-shadow: 0 0 8px rgba(248, 113, 113, 0.7); }
.dot-idle { background: var(--muted); }

.battery-icon { icon-size: 18px; }
.battery-ok { color: var(--ok); }
.battery-bad { color: var(--bad); }

/* ---------- навигация ---------- */

.nav {
    background: var(--bg-2);
    border-right-width: 1px;
    border-right-color: var(--border);
    padding: 18px 14px;
    width: 230px;
}
.nav-caption { color: var(--muted); font-size: 11px; font-weight: bold; letter-spacing: 2px; padding: 0px 10px 6px 10px; }

.nav-btn {
    background: transparent;
    color: var(--text-2);
    border-radius: 10px;
    padding: 11px 14px;
    font-size: 15px;
    icon-size: 20px;
    text-align: left;
    transition: background-color 150ms ease-out, color 150ms ease-out;
    &:hover { background: var(--surface-2); color: var(--text); }
    &:selected { background: var(--accent-soft); color: #ffffff; icon-color: var(--accent); }
}

.nav-hero { width: 200px; height: 125px; opacity: 0.9; }
.nav-foot { color: var(--muted); font-size: 11px; text-align: center; padding-top: 6px; }

/* ---------- контент ---------- */

.content {
    background: var(--bg);
    scrollbar-width: 8px;
    scrollbar-color: #2e3440;
    scrollbar-thumb-hover-color: #3d4556;
    scrollbar-track-color: transparent;
    scrollbar-radius: 4px;
}

.page { width: 100%; }

.page-title { color: var(--text); font-size: 28px; font-weight: bold; }
.page-sub { color: var(--muted); font-size: 14px; }

.card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 20px 22px;
}
.card-title { color: var(--text); font-size: 16px; font-weight: bold; }
.card-hint { color: var(--muted); font-size: 13px; }

.row-title { color: var(--text); font-size: 15px; font-weight: 600; }
.row-desc { color: var(--muted); font-size: 13px; }
.field-label { color: var(--text-2); font-size: 13px; font-weight: 600; }
.field-value { color: var(--text); font-size: 13px; font-weight: bold; }
.muted { color: var(--muted); font-size: 13px; }
.changed { color: var(--warn); }
.error-text { color: var(--bad); font-size: 13px; }

/* ---------- стандартные контролы ---------- */

Button {
    background: var(--surface-2);
    color: var(--text);
    border-radius: 10px;
    padding: 9px 16px;
    font-size: 14px;
    icon-size: 18px;
    transition: background-color 150ms ease-out;
    &:hover { background: var(--surface-3); }
    &:pressed { background: var(--border); }
    &:disabled { background: var(--surface); color: #555c6a; icon-color: #555c6a; }
}

.btn-primary {
    background: var(--accent);
    color: #ffffff;
    font-weight: bold;
    padding: 10px 22px;
    box-shadow: 0 6px 18px rgba(255, 59, 78, 0.3);
    &:hover { background: var(--accent-hover); }
    &:pressed { background: #d92c3e; }
    &:disabled { background: #2a2e38; color: #6b7280; icon-color: #6b7280; box-shadow: none; }
}

.btn-ghost {
    background: transparent;
    color: var(--text-2);
    border: 1px solid var(--border-strong);
    &:hover { background: var(--surface-2); color: var(--text); }
    &:disabled { background: transparent; color: #4b5260; border-color: var(--border); icon-color: #4b5260; }
}

ToolButton {
    background: var(--surface-2);
    color: var(--text-2);
    border-radius: 8px;
    padding: 6px;
    icon-size: 20px;
    &:hover { background: var(--surface-3); color: var(--text); }
}

Slider {
    height: 6px;
    background: #2a2f3b;
    color: var(--accent);
    accent-color: var(--accent);
    border-radius: 3px;
    label-color: var(--text);
    value-font-size: 13px;
    caret-color: var(--accent);
}
.dpi-slider { width: 100%; height: 8px; border-radius: 4px; }
.wide-slider { width: 100%; }

Toggle {
    width: 46px;
    height: 26px;
    background: #2e3440;
    color: #ffffff;
    accent-color: var(--accent);
    border-radius: 13px;
    transition: background-color 200ms ease-out;
    &:checked { background: var(--accent); }
}

Dropdown {
    background: var(--surface-2);
    color: var(--text);
    accent-color: var(--accent);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    padding: 9px 12px;
    font-size: 14px;
}

ColorPicker {
    background: var(--surface-2);
    color: var(--text);
    accent-color: var(--accent);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    font-size: 13px;
}

SegmentedButton {
    background: var(--surface-2);
    color: var(--text-2);
    accent-color: var(--accent);
    border-color: var(--border-strong);
    border-radius: 10px;
    font-size: 13px;
    height: 36px;
    --selected-background: #ff3b4e;
    --selected-color: #ffffff;
}

Tooltip { background: #2a303c; color: var(--text); font-size: 12px; border-radius: 6px; }

/* ---------- DPI ---------- */

.level-card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px 16px;
    min-width: 108px;
    transition: background-color 150ms ease-out, border-color 150ms ease-out;
    &:hover { background: var(--surface-3); border-color: var(--border-strong); }
}
.level-card-sel {
    background: var(--accent-soft);
    border-color: var(--accent);
    &:hover { background: var(--accent-soft); border-color: var(--accent-hover); }
}
.level-dot { width: 10px; height: 10px; border-radius: 5px; }
.level-name { color: var(--muted); font-size: 12px; }
.level-star { color: var(--warn); icon-size: 15px; }
.level-dpi { color: var(--text); font-size: 22px; font-weight: bold; }
.count-btn { padding: 8px; }

.editor-level { color: var(--muted); font-size: 13px; font-weight: bold; letter-spacing: 1px; }
.dpi-big { color: #ffffff; font-size: 52px; font-weight: bold; }
.dpi-unit { color: var(--accent); font-size: 20px; font-weight: bold; padding-bottom: 10px; }
.scale-label { color: var(--muted); font-size: 12px; }

.tag {
    background: rgba(251, 191, 36, 0.12);
    border: 1px solid rgba(251, 191, 36, 0.45);
    border-radius: 16px;
    padding: 6px 12px;
}
.tag-icon { color: var(--warn); icon-size: 16px; }
.tag-text { color: var(--warn); font-size: 13px; font-weight: bold; }

.chip {
    background: var(--surface-2);
    color: var(--text-2);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 6px 14px;
    font-size: 13px;
    &:hover { background: var(--surface-3); color: var(--text); }
}
.chip-on {
    background: var(--accent-soft);
    color: #ffffff;
    border-color: var(--accent);
    &:hover { background: var(--accent-soft); }
}

.color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 14px;
    border: 2px solid rgba(255, 255, 255, 0.12);
}
.color-swatch-on {
    border: 3px solid #ffffff;
    box-shadow: 0 0 10px rgba(255, 255, 255, 0.45);
}

/* ---------- подсветка ---------- */

.mode-tile {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 16px 8px;
    min-width: 110px;
    transition: background-color 150ms ease-out, border-color 150ms ease-out;
    &:hover { background: var(--surface-3); border-color: var(--border-strong); }
}
.mode-tile-on {
    background: var(--accent-soft);
    border-color: var(--accent);
    &:hover { background: var(--accent-soft); }
}
.mode-icon { icon-size: 28px; color: var(--text-2); }
.mode-label { color: var(--text); font-size: 13px; text-align: center; }

.preview-card { width: 330px; }
.led-preview-img { width: 286px; height: 179px; margin-top: 12px; }
.glow-bar {
    width: 230px;
    height: 10px;
    border-radius: 5px;
    margin-top: 4px;
}
.glow-rainbow {
    background: linear-gradient(90deg, #ff0040, #ffae00, #3dff5a, #00c8ff, #7a3dff, #ff00c8);
}
.preview-caption { color: var(--muted); font-size: 13px; padding-top: 12px; }

/* ---------- кнопки ---------- */

.scheme-card { padding: 8px; }
.scheme-img { width: 400px; height: 327px; }

.btn-row {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px 16px;
}
.btn-row-changed { border-color: rgba(251, 191, 36, 0.55); }
.num-badge {
    width: 32px;
    height: 32px;
    border-radius: 16px;
    background: #c81e2d;
}
.num-text { color: #ffffff; font-size: 15px; font-weight: bold; }

/* ---------- заглушка ---------- */

.ph-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 18px;
    padding: 36px 44px;
    margin-top: 60px;
    max-width: 620px;
}
.ph-badge {
    width: 72px;
    height: 72px;
    border-radius: 36px;
    background: var(--accent-soft);
    border: 1px solid var(--accent-line);
}
.ph-icon { color: var(--accent); icon-size: 36px; }
.ph-title { color: var(--text); font-size: 22px; font-weight: bold; }
.ph-hint { color: var(--text-2); font-size: 14px; text-align: center; line-height: 22px; }
.code-box { background: #0a0c10; border: 1px solid var(--border); border-radius: 10px; padding: 12px 16px; }
.code { color: #9fe3b8; font-size: 13px; font-family: monospace; }

/* ---------- нижняя панель ---------- */

.footer {
    background: var(--bg-2);
    border-top-width: 1px;
    border-top-color: var(--border);
    padding: 12px 22px;
}
.status-text { color: var(--text-2); font-size: 13px; }
.status-icon { icon-size: 18px; }
.status-ok { color: var(--ok); }
.status-bad { color: var(--bad); }
.status-info { color: var(--info); }

CircularProgress { color: var(--accent); accent-color: var(--accent); track-color: #2a2f3b; }
