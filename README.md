# ⌨️ LayoutKeeper

<p align="center">
  <img src="Demos/screenshotone.png" alt="LayoutKeeper v2.0 Screenshot" style="width: 100%; max-width: 820px; height: auto; display: block; margin: 0 auto; border-radius: 14px; box-shadow: 0 16px 40px rgba(0,0,0,0.4);">
</p>

<p align="center">
  <b>Smart, lightweight, and automated per-window keyboard layout manager for Linux.</b><br>
  <i>Written in native Rust &bull; Instant &lt; 1ms switching &bull; Zero idle CPU &bull; Under 3MB RAM</i>
</p>

<p align="center">
  <a href="https://github.com/MR-PR0G/LayoutKeeper/releases/latest"><img src="https://img.shields.io/badge/release-v2.0.0-blue.svg?style=flat-square" alt="Version"></a>
  <a href="#-supported-environments"><img src="https://img.shields.io/badge/wayland-niri%20%7C%20hyprland%20%7C%20sway%20%7C%20gnome%20%7C%20kde-success.svg?style=flat-square" alt="Wayland"></a>
  <a href="#-supported-environments"><img src="https://img.shields.io/badge/x11-i3%20%7C%20bspwm%20%7C%20awesome%20%7C%20xfce-blueviolet.svg?style=flat-square" alt="X11"></a>
  <a href="#-performance-benchmarks"><img src="https://img.shields.io/badge/memory-%3C%203MB%20RSS-brightgreen.svg?style=flat-square" alt="RAM"></a>
  <a href="#-performance-benchmarks"><img src="https://img.shields.io/badge/cpu-0.0%25%20idle-brightgreen.svg?style=flat-square" alt="CPU"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-orange.svg?style=flat-square" alt="License"></a>
</p>

---

## ⚡ Why LayoutKeeper?

Typing in multiple languages on Linux often means constantly hitting <kbd>Alt</kbd>+<kbd>Shift</kbd> by mistake: you chat in Persian, Arabic, or Russian in your browser or Telegram, switch to your terminal or editor, and accidentally type `در` or `سی‌دی` instead of `cd` or `ls`.

**LayoutKeeper fixes this permanently.** It runs quietly in the background and gives **every single window its own independent keyboard language memory**. When you return to a window, its layout is restored in less than a millisecond.

- 🚀 **Zero Configuration Required**: Auto-detects your compositor on launch.
- 🔋 **Zero Battery Drain**: 100% event-driven through native IPC sockets. **0.0% CPU when idle**.
- 🪶 **Featherweight**: Single standalone binary (< 1MB), using just ~3MB of RAM.
- 🎮 **Smart Exceptions**: Bypass games (Steam, Gamescope, Wine) and overlays with a single command.

---

## 🚀 Instant Installation (One Command)

Install and activate LayoutKeeper immediately:

```bash
curl -fsSL https://raw.githubusercontent.com/MR-PR0G/LayoutKeeper/main/install.sh | bash
```

> **What the installer does:**  
> Automatically detects your environment, downloads the pre-built native binary (or compiles with Cargo if preferred), creates the user configuration, and starts the background `systemd` user service immediately. **Zero root password required.**

### Or Download Pre-Built Binary Directly

Pre-compiled standalone binaries for **Linux x86_64** are available on the [**Releases Page**](https://github.com/MR-PR0G/LayoutKeeper/releases/latest):

```bash
# Download standalone binary
curl -sL https://github.com/MR-PR0G/LayoutKeeper/releases/latest/download/layoutkeeper -o ~/.local/bin/layoutkeeper
chmod +x ~/.local/bin/layoutkeeper

# Start service
layoutkeeper on
```

---

## 🖥️ Supported Environments & Prerequisites

LayoutKeeper talks directly to compositor IPC protocols — **no heavy dependencies needed**:

| Environment | Compositors / Desktop | Prerequisites | How it Works |
| :--- | :--- | :---: | :--- |
| **Wayland (Modern)** | **Niri**, **Hyprland**, **Sway** | **Zero dependencies** | Native IPC event stream & Unix sockets |
| **GNOME Shell** | Wayland & X11 | `gsettings` *(pre-installed)* | D-Bus & GSettings integration |
| **KDE Plasma** | Wayland & X11 | `qdbus` *(pre-installed)* | KDE Keyboard D-Bus interface |
| **X11 Window Managers** | **i3**, **Bspwm**, **AwesomeWM**, **XFCE**, etc. | `setxkbmap` | Native X11 event dispatcher *(auto-installed)* |

---

## 🎛️ Three Operating Modes

Choose how you want your keyboard to behave using `layoutkeeper mode <name>`:

1. **`per-window`** *(Default & Recommended)*:  
   Every window (Firefox, Telegram, LibreOffice, Terminal, Code Editor) independently remembers the exact layout you used in it.
2. **`hybrid`**:  
   Per-window memory across all desktop apps, but terminals always switch to **English (US)** automatically when focused.
3. **`terminal-focus`** *(Classic)*:  
   Switches to English (US) whenever you enter a terminal, and restores the previous language when you leave.

---

## 🎮 CLI Quick Reference

Manage the running daemon effortlessly from your terminal:

```bash
# View live status, active window, layout, and memory table
layoutkeeper status

# Toggle auto-switching on/off (bind this to a keyboard shortcut!)
layoutkeeper toggle

# Switch mode on the fly (no restart needed)
layoutkeeper mode per-window
layoutkeeper mode hybrid

# List tracked windows and their remembered layouts
layoutkeeper list

# Bypass games so LayoutKeeper never switches layouts inside them
layoutkeeper ignore steam
layoutkeeper ignore gamescope
layoutkeeper unignore steam

# Add / remove custom terminal emulators
layoutkeeper add wezterm
layoutkeeper remove alacritty

# View live daemon logs
layoutkeeper logs

# Stop / restart background daemon
layoutkeeper off
layoutkeeper restart
```

---

## ⌨️ Hotkey Integration

Bind `layoutkeeper toggle` to easily pause/resume layout auto-switching with a shortcut:

- **Niri** (`~/.config/niri/config.kdl`):
  ```kdl
  binds {
      Mod+Alt+K { spawn "layoutkeeper" "toggle"; }
  }
  ```
- **Hyprland** (`~/.config/hypr/hyprland.conf`):
  ```ini
  bind = $mainMod ALT, K, exec, layoutkeeper toggle
  ```
- **Sway / i3** (`~/.config/sway/config`):
  ```ini
  bindsym $mod+Mod1+k exec layoutkeeper toggle
  ```

---

## 📊 Performance Benchmarks

| Metric | **LayoutKeeper v2.0** | **GNOME Native** | **kbdd** | **Shell Script Deamons** |
| :--- | :---: | :---: | :---: | :---: |
| **Compositor Coverage** | **Universal (Wayland & X11)** | GNOME only | X11 only | Limited |
| **Memory Footprint** | **~2.8 MB RSS** | ~60+ MB | ~8 MB | Spawns multiple sub-processes |
| **Idle CPU Usage** | **0.0%** (Event-driven) | Variable | Low | High (Polling loops) |
| **Switch Latency** | **< 1 ms** | ~15 ms | ~8 ms | 100–250 ms |
| **Pre-built Binary** | **Yes (< 1MB Standalone)** | Built-in | Needs build | No |

---

## ⚙️ Configuration

Custom settings are saved in human-readable TOML format at `~/.config/layoutkeeper/config.toml`:

```toml
[general]
mode = "per-window"          # "per-window", "hybrid", or "terminal-focus"
default_layout = "0"         # Layout for newly opened windows
remember_per_window_id = true # Track each window instance separately
debug = false

[terminals]
terminals = ["kitty", "alacritty", "foot", "wezterm", "gnome-terminal", "konsole", "ghostty"]
us_layout = "0"

[rules]
ignored_apps = ["steam", "steam_app_.*", "gamescope", "rofi", "wofi", "swaylock", "hyprlock"]
```

---

## 🗑️ Uninstallation

To remove LayoutKeeper cleanly:

```bash
./install.sh --uninstall
```

---

## 🇮🇷 راهنمای سریع فارسی (Persian Quickstart)

پروژه **LayoutKeeper v2.0** یک دیمون سیستمی بسیار سبک، سریع و کاملاً بومی (Native Rust) برای لینوکس است که به هر پنجره، حافظه مستقل لایه کیبورد می‌دهد:

- **نصب فوق‌العاده سریع با یک دستور:**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/MR-PR0G/LayoutKeeper/main/install.sh | bash
  ```
- **بدون پیش‌نیاز سنگین:** در Wayland (نایری، هایپرلند، سوِی) مستقیماً به سوکت متصل شده و به هیچ پکیج اضافی نیاز ندارد.
- **مصرف بهینه:** کمتر از ۳ مگابایت رم، ۰٪ استفاده از سی‌پی‌یو در حالت بیکاری، و سوئیچ زیر یک میلی‌ثانیه.
- **تغییر حالت کاربری:**
  ```bash
  layoutkeeper mode per-window   # حافظه اختصاصی برای تمام پنجره‌ها
  layoutkeeper mode hybrid       # حافظه اختصاصی برای برنامه‌ها + انگلیسی خودکار در ترمینال‌ها
  ```
- **استثنا کردن بازی‌ها:** با دستور `layoutkeeper ignore steam` می‌توانید بازی‌ها را کاملاً مستثنی کنید.

---

## 📄 License

Distributed under the MIT License. Developed with ❤️ by [MR-PR0G](https://github.com/MR-PR0G).
