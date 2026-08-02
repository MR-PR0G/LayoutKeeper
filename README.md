# ⌨️ LayoutKeeper

<p align="center">
  <b>Smart, seamless, and automated keyboard layout switcher for Linux terminals.</b>
</p>

Never accidentally type `<ctrl42>` or `در` instead of `ls` or `cd` again! 

<p align="center">
  <img src="Demos/screenshotone.png" alt="LayoutKeeper Screenshot" style="width: 100%; max-width: 800px; height: auto; display: block; margin: 0 auto; border-radius: 8px;">
</p>

**LayoutKeeper** is a lightweight, background daemon for Linux that automatically switches your keyboard layout to **English (US)** whenever you focus a terminal application, and seamlessly **restores your previous layout** when you switch away.

---

## ✨ Features

- ⚡ **Blazing Fast**: Written in pure Bash with minimal overhead (< 2MB RAM usage).
- 🔄 **Automatic Restoration**: Remembers your exact layout (Persian, Russian, Arabic, German, etc.) and restores it upon leaving the terminal.
- 🎯 **Smart Window Tracking**: Real-time focus detection on both Wayland compositors and X11 WMs.
- 🛠️ **Terminal Auto-Detection**: Scans your system and lets you select all, one, or custom terminal emulators during setup.
- ⚙️ **Systemd Integration**: Runs automatically in user-space with background recovery.
- 📦 **Zero Complex Dependencies**: Uses standard native system tools (`jq`, `xdotool`, DBus/compositor IPCs).

---

## 🖥️ Supported Environments

LayoutKeeper auto-detects your display server and window compositor on boot:

| Display Server | Compositors / WMs / DEs |
| :--- | :--- |
| **Wayland** | Niri, Hyprland, Sway, GNOME Shell, KDE Plasma |
| **X11** | i3, Bspwm, AwesomeWM, Openbox, XFCE, Cinnamon, MATE, etc. |

### Supported Terminals
Auto-detects: `kitty`, `alacritty`, `konsole`, `foot`, `wezterm`, `gnome-terminal`, `tilix`, `terminator`, `xterm`, `urxvt`, `xfce4-terminal`, `st`, `qterminal`, and custom user inputs.

---

## 🚀 Installation

### Interactive One-Step Installer

Install LayoutKeeper instantly using bash without cloning the repository manually:

```bash
curl -fsSL https://raw.githubusercontent.com/MR-PR0G/LayoutKeeper/main/install.sh | bash
```
### Or
Clone the repository and run the setup script:

```bash
git clone https://github.com/your-username/layoutkeeper.git
cd layoutkeeper
chmod +x install.sh
./install.sh
```

The installer will:
1. Display your system information and detected display server.
2. Scan and list all installed terminals.
3. Allow you to choose which terminals to manage.
4. Set up dependencies and start the `systemd` user service automatically.

---

## 🎮 Usage

LayoutKeeper runs headlessly in the background, but you can control it using the `layoutkeeper` command-line interface:

```bash
# Check service status
layoutkeeper status

# View live system logs (helpful for debugging)
layoutkeeper logs

# Temporarily disable / enable auto-switching
layoutkeeper off
layoutkeeper on

# Restart the daemon
layoutkeeper restart

# List currently monitored terminals
layoutkeeper list

# Add or remove terminals on the fly
layoutkeeper add wezterm
layoutkeeper remove alacritty

# Show help menu
layoutkeeper help
```

---

## ⚙️ Configuration

Your preferences are saved in `~/.config/layoutkeeper/config`:

```ini
[LayoutKeeper]
enabled = true
debug = false
restore_layout = true
terminals = kitty,alacritty,konsole
us_index = 0
restore_index = 1
```

- **`terminals`**: Comma-separated list of terminal process names or window classes.
- **`debug`**: Set to `true` to output focus change events to `journalctl`.
- **`us_index` / `restore_index`**: Configurable layout index offsets for index-based compositors like Niri.

---

## 🗑️ Uninstallation

To remove LayoutKeeper and disable the background service cleanly:

```bash
./install.sh --uninstall
```

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
