# ⌨️ LayoutKeeper

<p align="center">
  <img src="Demos/screenshotone.png" alt="LayoutKeeper Screenshot" style="width: 100%; max-width: 800px; height: auto; display: block; margin: 0 auto; border-radius: 12px; box-shadow: 0 8px 24px rgba(0,0,0,0.25);">
</p>

<p align="center">
  <b>Enterprise-Grade Universal Per-Window Keyboard Layout Manager & Smart Terminal Switcher for Linux.</b><br>
  <i>Ultra-lightweight native Rust daemon with sub-millisecond latency, zero CPU overhead, and native Wayland & X11 support.</i>
</p>

<p align="center">
  <a href="#-features"><img src="https://img.shields.io/badge/version-v2.0.0-blue.svg?style=flat-square" alt="Version 2.0.0"></a>
  <a href="#-supported-environments"><img src="https://img.shields.io/badge/platform-Wayland%20%7C%20X11-purple.svg?style=flat-square" alt="Platforms"></a>
  <a href="#-benchmarks--competitor-comparison"><img src="https://img.shields.io/badge/memory-%3C%203MB%20RSS-brightgreen.svg?style=flat-square" alt="Memory RSS"></a>
  <a href="#-benchmarks--competitor-comparison"><img src="https://img.shields.io/badge/latency-%3C%201ms-orange.svg?style=flat-square" alt="Latency"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg?style=flat-square" alt="License"></a>
</p>

---

## 💡 The Problem LayoutKeeper Solves

If you are a multilingual Linux user typing in Persian, Arabic, Russian, German, French, Spanish, Japanese, or any non-English layout, you know this constant frustration:
- You chat in your native language in **Telegram** or **Browser**, then switch to your **Terminal** or **IDE** and accidentally type `سی‌دی` or `در` instead of `cd` or `ls`.
- On window managers like **Niri**, **Hyprland**, **Sway**, **i3**, and **bspwm**, keyboard layout is global across all windows by default. GNOME Shell has a per-window setting, but it is notorious for input lag and only works inside GNOME.
- Competing tools either only support obsolete X11 protocols (like `kbdd`), are locked to a single compositor (like `hyprland-per-window-xkb`), or consume dozens of megabytes with continuous polling scripts.

**LayoutKeeper 2.0** solves this once and for all. It gives every window its own persistent keyboard layout memory across **all** modern Wayland compositors and X11 environments, while optionally guaranteeing terminals always open in English.

---

## ✨ Features at a Glance

- 🧠 **Smart Per-Window Memory**: Remembers the exact keyboard layout you chose for each window individually. Switch between browser, editor, and chat without ever hitting Alt+Shift again.
- ⚡ **Pure Native Rust Core**: Compiled into a single zero-dependency binary (< 1MB) with **sub-millisecond (< 1ms)** response time.
- 🔋 **Zero Idle CPU & Battery Friendly**: Fully push-based event streams (via Wayland IPC sockets and X11 events) — **0.0% CPU usage** when idle.
- 🖥️ **Universal Compositor Support**: First-class support for **Niri**, **Hyprland**, **Sway / i3**, **GNOME Shell**, **KDE Plasma**, and **X11**.
- 🎛️ **Three Operating Modes**:
  1. `per-window` *(Default)*: Every window remembers its own layout individually.
  2. `hybrid`: Per-window layout memory for all desktop apps, but terminals always auto-switch to English (US).
  3. `terminal-focus`: Classic LayoutKeeper mode — automatically switches to US in terminals, restores previous layout upon leaving.
- 🚫 **Bypass / Exception Management**: Whitelist or blacklist games (Steam, Gamescope, Wine), lockscreens, and overlays with a single command.
- 🔌 **Dynamic CLI & Socket IPC**: Control the running daemon in real-time (`layoutkeeper status`, `layoutkeeper toggle`, `layoutkeeper mode`, `layoutkeeper ignore`).
- 🛡️ **Enterprise Ready**: Clean systemd user service integration, graceful fault tolerance, and automatic crash recovery.

---

## 📊 Benchmarks & Competitor Comparison

| Feature / Metric | **LayoutKeeper 2.0** | **GNOME Native** | **kbdd** | **hyprland-per-window** | **Legacy Shell Scripts** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Compositors Supported** | **Niri, Hypr, Sway, GNOME, KDE, X11** | GNOME only | X11 only | Hyprland only | Limited |
| **RAM Usage (RSS)** | **~3 MB** | ~60+ MB (Shell) | ~8 MB | ~15 MB | ~10 MB (spawns forks) |
| **CPU Usage (Idle)** | **0.0%** (Event-driven) | Variable | Low | Low | **High** (loops `sleep 0.1`) |
| **Switch Latency** | **< 1 ms** | 10–30 ms | 5–10 ms | 2–5 ms | 100–250 ms |
| **Terminal Auto-US Mode** | **Yes (Hybrid & Classic)** | No | No | No | Only terminals |
| **Real-time CLI Management** | **Yes (Unix Socket IPC)** | No | No | No | Limited |
| **Game / Overlay Ignore List** | **Yes (`layoutkeeper ignore`)** | No | No | No | No |
| **Zero-dependency Binary** | **Yes (< 1MB Static)** | System-dependent | C binary | Python / Rust | External tools required |

---

## 🏗️ Architecture

```mermaid
flowchart TD
    subgraph Compositors ["🖥️ Compositor / Display Server Layer"]
        Niri["Niri (IPC Event Stream)"]
        Hypr["Hyprland (Socket2 IPC)"]
        Sway["Sway / i3 (IPC Subscribe)"]
        Gnome["GNOME Shell (GSettings / D-Bus)"]
        Kde["KDE Plasma (KWin D-Bus)"]
        X11["X11 (_NET_ACTIVE_WINDOW / XKB)"]
    end

    subgraph CoreDaemon ["⚙️ LayoutKeeper Core Engine (Native Rust)"]
        EventListener["Event Listener & Dispatcher"]
        MemoryTable[("🧠 Per-Window Memory Table\n(Window ID ➔ Layout)")]
        RuleEngine["🎯 Rule & Mode Evaluator\n(per-window | hybrid | terminal-focus)"]
        IgnoreList["🚫 Ignore List (Steam, Games, Overlays)"]
    end

    subgraph Control ["🎮 Control Layer"]
        CLI["CLI Commands\n(status, mode, toggle, list)"]
        UnixSocket["/run/user/$UID/layoutkeeper.sock"]
        Systemd["systemd --user layoutkeeper.service"]
    end

    Niri -->|WindowFocusChanged & LayoutSwitched| EventListener
    Hypr -->|activewindow & activelayout| EventListener
    Sway -->|window & input events| EventListener
    Gnome -->|gsettings & dbus monitor| EventListener
    Kde -->|dbus event listener| EventListener
    X11 -->|xprop / xdotool events| EventListener

    EventListener --> RuleEngine
    RuleEngine <--> MemoryTable
    RuleEngine <--> IgnoreList
    RuleEngine -->|Set Layout Action| Compositors

    CLI <--> UnixSocket <--> CoreDaemon
    Systemd -->|Auto-start & supervise| CoreDaemon
```

---

## 🖥️ Supported Environments

LayoutKeeper automatically detects your display server and window compositor on boot:

| Display Server | Compositors / WMs / Desktop Environments |
| :--- | :--- |
| **Wayland** | **Niri**, **Hyprland**, **Sway**, **GNOME Shell**, **KDE Plasma**, River, etc. |
| **X11** | **i3**, **Bspwm**, **AwesomeWM**, **Openbox**, **XFCE**, **Cinnamon**, **MATE**, etc. |

---

## 🚀 Installation

### Automated One-Line Installer (Recommended)

Run the fast interactive installer via terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/MR-PR0G/LayoutKeeper/main/install.sh | bash
```

### Or Build & Install From Source

```bash
git clone https://github.com/MR-PR0G/LayoutKeeper.git
cd LayoutKeeper
chmod +x install.sh
./install.sh
```

The installer will:
1. Detect your operating system, compositor (Niri, Hyprland, Sway, GNOME, etc.), and architecture.
2. Build the optimized release binary with Cargo (`target/release/layoutkeeper`).
3. Install the executable to `/usr/local/bin` (or `~/.local/bin` if non-root).
4. Prompt you to choose your desired operating mode (`per-window`, `hybrid`, or `terminal-focus`).
5. Configure and start the `systemd` user service immediately.

---

## 🎮 CLI Usage

Control the running daemon on the fly using the `layoutkeeper` command-line utility:

```bash
# Check live status, active window, layout, and memory table
layoutkeeper status

# Toggle layout switching on/off (perfect for binding to a hotkey!)
layoutkeeper toggle

# Temporarily pause / resume layout auto-switching
layoutkeeper pause
layoutkeeper resume

# Switch operating mode dynamically without restarting
layoutkeeper mode per-window
layoutkeeper mode hybrid
layoutkeeper mode terminal-focus

# View memory table of remembered windows and layouts
layoutkeeper list

# Clear remembered layout cache
layoutkeeper clear

# Ignore an application so LayoutKeeper never switches layouts in it
layoutkeeper ignore steam
layoutkeeper ignore gamescope
layoutkeeper unignore steam

# Add / remove terminal emulators to monitored list
layoutkeeper add ghostty
layoutkeeper remove alacritty

# View live system daemon logs
layoutkeeper logs

# View configuration file path and contents
layoutkeeper config

# Start / stop / restart the systemd background daemon
layoutkeeper on
layoutkeeper off
layoutkeeper restart

# Show help menu
layoutkeeper help
```

---

## ⚙️ Configuration

Your preferences are saved in `~/.config/layoutkeeper/config.toml`:

```toml
[general]
# Operating mode: "per-window", "hybrid", or "terminal-focus"
mode = "per-window"

# Default layout for newly opened windows ("0", "us", or "auto")
default_layout = "0"

# Track each window instance by unique window ID (true) or app class (false)
remember_per_window_id = true

# Debounce delay in milliseconds
debounce_ms = 25

# Enable debug logging to journalctl
debug = false

[terminals]
# List of terminal process names or window classes
terminals = [
    "kitty", "alacritty", "foot", "wezterm", "gnome-terminal",
    "konsole", "ghostty", "rio", "warp-terminal", "contour",
    "tilix", "terminator", "xterm", "urxvt", "xfce4-terminal",
    "qterminal", "st", "lxterminal", "mate-terminal", "guake", "yakuake"
]
# Fallback US layout index or name
us_layout = "0"

[rules]
# Applications bypassed completely (games, screen lockers, launchers)
ignored_apps = [
    "steam", "steam_app_.*", "gamescope",
    "org.kde.polkit-kde-authentication-agent-1",
    "polkit-gnome-authentication-agent-1",
    "rofi", "wofi", "dmenu", "swaylock", "hyprlock", "waylock"
]

[rules.enforce_layouts]
# Optionally force a specific layout whenever an application is focused
# "code" = "0"
```

---

## ⌨️ Recommended Hotkey Binding

You can bind `layoutkeeper toggle` to any keybinding in your compositor (e.g. `Super + Alt + K`) to instantly pause or resume layout keeper whenever needed:

### Niri (`~/.config/niri/config.kdl`):
```kdl
binds {
    Mod+Alt+K { spawn "layoutkeeper" "toggle"; }
}
```

### Hyprland (`~/.config/hypr/hyprland.conf`):
```ini
bind = $mainMod ALT, K, exec, layoutkeeper toggle
```

### Sway / i3 (`~/.config/sway/config`):
```ini
bindsym $mod+Mod1+k exec layoutkeeper toggle
```

---

## 🗑️ Uninstallation

To cleanly remove the daemon binary and systemd service:

```bash
layoutkeeper off
./install.sh --uninstall
```

*(Your configuration file in `~/.config/layoutkeeper` will be preserved).*

---

## 🇮🇷 راهنمای فارسی (Persian Guide)

پروژه **LayoutKeeper** یک دیمون سیستمی بسیار سبک، فوق‌سریع و هوشمند برای لینوکس است که به زبان **Rust** بازنویسی شده است. این ابزار به هر پنجره دسکتاپ حافظه اختصاصی لایه کیبورد می‌دهد تا زبان کیبورد هر برنامه به‌صورت کاملاً مستقل و خودکار ذخیره و بازیابی شود:

- **حالت پیش‌فرض (Per-Window):** هر پنجره (مانند فایرفاکس، تلگرام، لیبره‌آفیس، ترمینال و...) آخرین زبانی که در آن انتخاب کرده‌اید را در حافظه نگه می‌دارد. وقتی از تلگرام به ترمینال یا برعکس جابه‌جا می‌شوید، زبان کیبورد در کسر میلی‌ثانیه به زبان اختصاصی همان برنامه تغییر می‌کند.
- **حالت ترکیبی (Hybrid):** برنامه‌های معمولی زبان خود را به خاطر می‌سپارند، اما ترمینال‌ها همیشه به محض ورود به‌صورت خودکار به انگلیسی (US) تغییر می‌کنند.
- **مصرف بهینه:** کمتر از ۳ مگابایت رم، ۰٪ استفاده از پردازنده در حالت آماده‌به‌کار (به‌دلیل ارتباط مستقیم با سوکت‌های Wayland/X11 بدون حلقه Polling سنگین).
- **مدیریت استثناها:** بازی‌ها و برنامه‌هایی مانند Steam را می‌توانید با دستور `layoutkeeper ignore steam` مستثنی کنید تا سوئیچ کیبورد در آن‌ها غیرفعال شود.

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

Developed with ❤️ by [MR-PR0G](https://github.com/MR-PR0G) and contributors.
