#!/usr/bin/env bash
# ==============================================================================
# LayoutKeeper v2.0 — Universal Keyboard Layout Manager Installer
# https://github.com/MR-PR0G/LayoutKeeper
# ==============================================================================

set -eo pipefail

APP_NAME="layoutkeeper"
VERSION="2.0.0"
REPO_URL="https://github.com/MR-PR0G/LayoutKeeper"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/$APP_NAME"
SYSTEMD_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

# Colors
BOLD="\033[1m"
DIM="\033[2m"
RED="\033[31m"
GREEN="\033[32m"
YELLOW="\033[33m"
BLUE="\033[34m"
MAGENTA="\033[35m"
CYAN="\033[36m"
RESET="\033[0m"

print_banner() {
    clear 2>/dev/null || true
    echo -e "${CYAN}${BOLD}"
    cat << "EOF"
    __                            __  __ __                           
   / /   ____ ___  ______  __  __/ /_/ //_/__  ___  ____  ___  _____  
  / /   / __ `/ / / / __ \/ / / / __/ ,< / _ \/ _ \/ __ \/ _ \/ ___/  
 / /___/ /_/ / /_/ / /_/ / /_/ / /_/ /| |  __/  __/ /_/ /  __/ /      
/_____/\__,_/\__, /\____/\__,_/\__/_/ |_|\___/\___/ .___/\___/_/       
            /____/                               /_/                  
EOF
    echo -e "${RESET}"
    echo -e "                   ${BOLD}Version: ${YELLOW}v$VERSION (Next-Gen Native)${RESET}"
    echo -e "        Universal Per-Window Keyboard Layout Manager for Linux\n"
}

detect_environment() {
    OS_NAME="Linux"
    if [ -f /etc/os-release ]; then
        OS_NAME=$(grep -E "^PRETTY_NAME=" /etc/os-release | cut -d'=' -f2 | tr -d '"')
    fi

    ARCH=$(uname -m)

    if [ -n "$WAYLAND_DISPLAY" ]; then
        DISP_SERVER="Wayland"
        if [ -n "$HYPRLAND_INSTANCE_SIGNATURE" ]; then
            COMPOSITOR="Hyprland"
        elif [ -n "$SWAYSOCK" ]; then
            COMPOSITOR="Sway"
        elif [ -n "$NIRI_SOCKET" ] || command -v niri &>/dev/null && pgrep -x niri &>/dev/null; then
            COMPOSITOR="Niri"
        elif [[ "${XDG_CURRENT_DESKTOP,,}" == *"gnome"* ]]; then
            COMPOSITOR="GNOME Shell"
        elif [[ "${XDG_CURRENT_DESKTOP,,}" == *"kde"* ]]; then
            COMPOSITOR="KDE Plasma"
        else
            COMPOSITOR="${XDG_CURRENT_DESKTOP:-Generic Wayland}"
        fi
    elif [ -n "$DISPLAY" ]; then
        DISP_SERVER="X11"
        COMPOSITOR="${XDG_CURRENT_DESKTOP:-X11 WM}"
    else
        DISP_SERVER="Headless / None"
        COMPOSITOR="N/A"
    fi
}

print_sysinfo() {
    echo -e "${BOLD}[System Environment]${RESET}"
    echo -e "  Operating System : ${CYAN}$OS_NAME ($ARCH)${RESET}"
    echo -e "  Display Server    : ${YELLOW}$DISP_SERVER${RESET}"
    echo -e "  Compositor / WM   : ${GREEN}$COMPOSITOR${RESET}"
    echo -e " 📦 Config Directory : ${DIM}$CONFIG_DIR${RESET}\n"
}

determine_bin_dir() {
    # Check if user has root/sudo write access to /usr/local/bin
    if [ "$EUID" -eq 0 ]; then
        TARGET_BIN_DIR="/usr/local/bin"
        USE_SUDO=""
    elif [ -w "/usr/local/bin" ]; then
        TARGET_BIN_DIR="/usr/local/bin"
        USE_SUDO=""
    elif sudo -n true 2>/dev/null; then
        TARGET_BIN_DIR="/usr/local/bin"
        USE_SUDO="sudo"
    else
        # Non-root fallback: user-space installation
        TARGET_BIN_DIR="$HOME/.local/bin"
        USE_SUDO=""
        mkdir -p "$TARGET_BIN_DIR"
        
        # Ensure ~/.local/bin is in PATH for current session
        if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
            export PATH="$HOME/.local/bin:$PATH"
        fi
    fi
}

uninstall() {
    echo -e "\n${YELLOW}=> Uninstalling LayoutKeeper...${RESET}"
    systemctl --user stop layoutkeeper.service 2>/dev/null || true
    systemctl --user disable layoutkeeper.service 2>/dev/null || true
    rm -f "$SYSTEMD_DIR/layoutkeeper.service"
    systemctl --user daemon-reload 2>/dev/null || true

    rm -f "/usr/local/bin/layoutkeeper" "$HOME/.local/bin/layoutkeeper"
    echo -e "${GREEN}✔ LayoutKeeper binary and systemd service removed.${RESET}"
    echo -e "${DIM}Note: Your configuration remains in $CONFIG_DIR.${RESET}\n"
    exit 0
}

NON_INTERACTIVE=false
for arg in "$@"; do
    case "$arg" in
        --uninstall|-u)
            uninstall
            ;;
        --yes|-y|--non-interactive)
            NON_INTERACTIVE=true
            ;;
    esac
done

build_and_install_binary() {
    echo -e "${BOLD}[Installing LayoutKeeper Native Binary]${RESET}"

    determine_bin_dir
    echo -e " Installation path: ${CYAN}$TARGET_BIN_DIR/layoutkeeper${RESET}"

    # 1. Local compiled binary in release target
    if [ -f "./target/release/layoutkeeper" ]; then
        echo -e "${GREEN}Found locally compiled release binary!${RESET}"
        $USE_SUDO cp "./target/release/layoutkeeper" "$TARGET_BIN_DIR/layoutkeeper"
    # 2. Try fetching pre-compiled release binary from GitHub Releases
    elif [ "$FORCE_BUILD" != "true" ] && curl -sSfL "https://github.com/MR-PR0G/LayoutKeeper/releases/download/v$VERSION/layoutkeeper" -o /tmp/layoutkeeper-bin 2>/dev/null; then
        echo -e "${GREEN}Downloaded pre-built binary from GitHub Releases (v$VERSION)!${RESET}"
        $USE_SUDO cp /tmp/layoutkeeper-bin "$TARGET_BIN_DIR/layoutkeeper"
        rm -f /tmp/layoutkeeper-bin
    # 3. Local Cargo build
    elif [ -f "./Cargo.toml" ] && command -v cargo &>/dev/null; then
        echo -e "${BLUE}Building release binary with Cargo (optimized, LTO enabled)...${RESET}"
        cargo build --release --locked || cargo build --release
        $USE_SUDO cp "./target/release/layoutkeeper" "$TARGET_BIN_DIR/layoutkeeper"
    # 4. Clone and build with Cargo
    elif command -v cargo &>/dev/null; then
        echo -e "${BLUE}Building native binary from source...${RESET}"
        TEMP_DIR=$(mktemp -d)
        git clone --depth 1 "$REPO_URL.git" "$TEMP_DIR"
        (cd "$TEMP_DIR" && cargo build --release)
        $USE_SUDO cp "$TEMP_DIR/target/release/layoutkeeper" "$TARGET_BIN_DIR/layoutkeeper"
        rm -rf "$TEMP_DIR"
    else
        echo -e "${RED}Error: Pre-built binary unavailable and Cargo is not installed.${RESET}"
        echo -e "To install Cargo: ${CYAN}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${RESET}"
        exit 1
    fi

    $USE_SUDO chmod +x "$TARGET_BIN_DIR/layoutkeeper"
    echo -e "${GREEN}✔ Binary successfully installed to $TARGET_BIN_DIR/layoutkeeper${RESET}\n"
}

select_mode() {
    echo -e "${BOLD}[Operating Mode Configuration]${RESET}"
    echo -e "Choose how LayoutKeeper should manage your keyboard layouts:"
    echo -e "  1) ${GREEN}${BOLD}Per-Window Memory${RESET} (Recommended)"
    echo -e "     Every single window retains its own keyboard layout automatically."
    echo -e "  2) ${CYAN}${BOLD}Hybrid Mode${RESET}"
    echo -e "     Per-window memory for regular apps, but terminals always force US layout."
    echo -e "  3) ${YELLOW}${BOLD}Terminal Focus Only${RESET} (Classic v1 behavior)"
    echo -e "     Only switches to US when terminal is focused; restores layout when leaving."

    SELECTED_MODE="per-window"
    if [ "$NON_INTERACTIVE" != "true" ] && [ -t 0 ] && [ -r /dev/tty ]; then
        read -p "Select mode [1/2/3] (default: 1): " MODE_CHOICE < /dev/tty || MODE_CHOICE="1"
        case "$MODE_CHOICE" in
            2) SELECTED_MODE="hybrid" ;;
            3) SELECTED_MODE="terminal-focus" ;;
            *) SELECTED_MODE="per-window" ;;
        esac
    fi
    echo -e "✔ Mode selected: ${MAGENTA}${BOLD}$SELECTED_MODE${RESET}\n"
}

setup_configuration() {
    mkdir -p "$CONFIG_DIR"
    CONFIG_FILE="$CONFIG_DIR/config.toml"

    if [ ! -f "$CONFIG_FILE" ]; then
        echo -e "${BLUE}Writing default configuration to ${CYAN}$CONFIG_FILE${RESET}..."
        cat > "$CONFIG_FILE" << EOF
# ==============================================================================
# LayoutKeeper Configuration (v2.0)
# Documentation: https://github.com/MR-PR0G/LayoutKeeper
# ==============================================================================

[general]
mode = "$SELECTED_MODE"
default_layout = "0"
remember_per_window_id = true
debounce_ms = 25
debug = false

[terminals]
terminals = [
    "kitty", "alacritty", "foot", "wezterm", "gnome-terminal",
    "konsole", "ghostty", "rio", "warp-terminal", "contour",
    "tilix", "terminator", "xterm", "urxvt", "xfce4-terminal",
    "qterminal", "st", "lxterminal", "mate-terminal", "guake", "yakuake"
]
us_layout = "0"

[rules]
ignored_apps = [
    "steam", "steam_app_.*", "gamescope",
    "org.kde.polkit-kde-authentication-agent-1",
    "polkit-gnome-authentication-agent-1",
    "rofi", "wofi", "dmenu", "swaylock", "hyprlock", "waylock"
]

[rules.enforce_layouts]
# Example: enforce US on specific app
# "code" = "0"
EOF
    else
        # Update existing config's mode if chosen
        sed -i "s/^mode = .*/mode = \"$SELECTED_MODE\"/" "$CONFIG_FILE" 2>/dev/null || true
    fi
}

setup_systemd_service() {
    echo -e "${BOLD}[Systemd User Service Setup]${RESET}"
    mkdir -p "$SYSTEMD_DIR"
    SERVICE_FILE="$SYSTEMD_DIR/layoutkeeper.service"

    cat > "$SERVICE_FILE" << EOF
[Unit]
Description=LayoutKeeper - Universal Keyboard Layout Manager
Documentation=https://github.com/MR-PR0G/LayoutKeeper
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$TARGET_BIN_DIR/layoutkeeper daemon
Restart=on-failure
RestartSec=2
KillMode=process

[Install]
WantedBy=graphical-session.target default.target
EOF

    echo -e " Reloading systemd user daemon..."
    systemctl --user daemon-reload
    systemctl --user enable layoutkeeper.service
    systemctl --user restart layoutkeeper.service
    echo -e "${GREEN}✔ Service layoutkeeper.service enabled and started in background.${RESET}\n"
}

check_dependencies() {
    if [ "$DISP_SERVER" = "X11" ]; then
        local missing=()
        if ! command -v setxkbmap &>/dev/null; then missing+=("setxkbmap"); fi
        if ! command -v xdotool &>/dev/null; then missing+=("xdotool"); fi
        if [ ${#missing[@]} -gt 0 ]; then
            echo -e "${YELLOW}Notice: Installing missing X11 helper tools (${missing[*]})...${RESET}"
            if command -v apt-get &>/dev/null && [ -n "$USE_SUDO" -o "$EUID" -eq 0 ]; then
                $USE_SUDO apt-get update -qq 2>/dev/null || true
                $USE_SUDO apt-get install -y -qq x11-xkb-utils xdotool 2>/dev/null || true
            elif command -v pacman &>/dev/null && [ -n "$USE_SUDO" -o "$EUID" -eq 0 ]; then
                $USE_SUDO pacman -Sy --noconfirm xorg-setxkbmap xdotool 2>/dev/null || true
            elif command -v dnf &>/dev/null && [ -n "$USE_SUDO" -o "$EUID" -eq 0 ]; then
                $USE_SUDO dnf install -y xorg-x11-server-utils xdotool 2>/dev/null || true
            fi
        fi
    fi
}

# ----------------- MAIN FLOW -----------------
print_banner
detect_environment
print_sysinfo
select_mode
determine_bin_dir
check_dependencies
build_and_install_binary
setup_configuration
setup_systemd_service

sleep 0.5

echo -e "${GREEN}${BOLD}═══════════════════════════════════════════════════════════════${RESET}"
echo -e "${GREEN}${BOLD}  ✔ LayoutKeeper v$VERSION is successfully installed & running!${RESET}"
echo -e "${GREEN}${BOLD}═══════════════════════════════════════════════════════════════${RESET}\n"

# Verify status using the newly installed CLI
"$TARGET_BIN_DIR/layoutkeeper" status || true

echo -e "\n${BOLD}Quick commands:${RESET}"
echo -e "  • Check status : ${CYAN}layoutkeeper status${RESET}"
echo -e "  • Switch mode  : ${CYAN}layoutkeeper mode <per-window | hybrid | terminal-focus>${RESET}"
echo -e "  • Toggle pause : ${CYAN}layoutkeeper toggle${RESET}"
echo -e "  • View memory  : ${CYAN}layoutkeeper list${RESET}"
echo -e "  • Ignore game  : ${CYAN}layoutkeeper ignore <app>${RESET}"
echo -e "  • Help menu    : ${CYAN}layoutkeeper help${RESET}\n"
