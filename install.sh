#!/bin/bash

set -e

APP_NAME="layoutkeeper"
VERSION="1.1.0"
REPO_RAW_URL="https://raw.githubusercontent.com/MR-PR0G/LayoutKeeper/main"
BIN_DIR="/usr/local/bin"
CONFIG_DIR="$HOME/.config/$APP_NAME"
SYSTEMD_DIR="$HOME/.config/systemd/user"

BOLD="\e[1m"
RED="\e[31m"
GREEN="\e[32m"
YELLOW="\e[33m"
BLUE="\e[34m"
CYAN="\e[36m"
RESET="\e[0m"

print_banner() {
    clear
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
    echo -e "                 ${BOLD}Version: ${YELLOW}$VERSION${RESET}"
    echo -e "      Smart Keyboard Layout Switcher for Terminals\n"
}

detect_system() {
    OS_NAME="Unknown"
    if [ -f /etc/os-release ]; then
        OS_NAME=$(grep -E "^PRETTY_NAME=" /etc/os-release | cut -d'=' -f2 | tr -d '"')
    fi

    if [ -n "$WAYLAND_DISPLAY" ]; then
        DISP_SERVER="Wayland"
        if [ -n "$HYPRLAND_INSTANCE_SIGNATURE" ]; then
            COMPOSITOR="Hyprland"
            LAYOUT_TOOL="hyprctl"
        elif [ -n "$SWAYSOCK" ]; then
            COMPOSITOR="Sway"
            LAYOUT_TOOL="swaymsg"
        elif [ -n "$NIRI_SOCKET" ]; then
            COMPOSITOR="Niri"
            LAYOUT_TOOL="niri msg"
        elif [[ "${XDG_CURRENT_DESKTOP,,}" == *"gnome"* ]]; then
            COMPOSITOR="GNOME"
            LAYOUT_TOOL="gsettings"
        elif [[ "${XDG_CURRENT_DESKTOP,,}" == *"kde"* ]]; then
            COMPOSITOR="KDE Plasma"
            LAYOUT_TOOL="qdbus"
        else
            COMPOSITOR="${XDG_CURRENT_DESKTOP:-Unknown Wayland}"
            LAYOUT_TOOL="generic"
        fi
    elif [ -n "$DISPLAY" ]; then
        DISP_SERVER="X11"
        COMPOSITOR="${XDG_CURRENT_DESKTOP:-Unknown WM/DE}"
        LAYOUT_TOOL="setxkbmap"
    else
        DISP_SERVER="None Detected"
        COMPOSITOR="N/A"
        LAYOUT_TOOL="N/A"
    fi

    if command -v dnf &> /dev/null; then PKG_MNGR="dnf"
    elif command -v pacman &> /dev/null; then PKG_MNGR="pacman"
    elif command -v apt-get &> /dev/null; then PKG_MNGR="apt-get"
    elif command -v zypper &> /dev/null; then PKG_MNGR="zypper"
    else PKG_MNGR="Unknown"
    fi
}

print_sysinfo() {
    echo -e "${BOLD}[System Information]${RESET}"
    echo -e "  OS Name         : ${CYAN}$OS_NAME${RESET}"
    echo -e "  Display Server  : ${YELLOW}$DISP_SERVER${RESET}"
    echo -e "  Compositor/WM   : ${GREEN}$COMPOSITOR${RESET}"
    echo -e "  Layout Manager  : ${BLUE}$LAYOUT_TOOL${RESET}"
    echo -e "  Package Manager : ${YELLOW}$PKG_MNGR${RESET}\n"
}

select_terminals() {
    echo -e "${BOLD}[Terminal Detection]${RESET}"
    echo -e "Scanning system for installed terminals...\n"

    KNOWN_TERMS=("kitty" "alacritty" "gnome-terminal" "konsole" "foot" "wezterm" "tilix" "terminator" "xterm" "urxvt" "xfce4-terminal" "qterminal" "st" "lxterminal")
    DETECTED=()

    for term in "${KNOWN_TERMS[@]}"; do
        if command -v "$term" &> /dev/null; then
            DETECTED+=("$term")
        fi
    done

    if [ ${#DETECTED[@]} -eq 0 ]; then
        echo -e "${YELLOW}No common terminals automatically detected.${RESET}"
        DETECTED=("kitty" "gnome-terminal" "alacritty")
    else
        echo -e "${GREEN}Detected the following terminals on your system:${RESET}"
        for i in "${!DETECTED[@]}"; do
            echo -e "  $((i+1)). ${CYAN}${DETECTED[$i]}${RESET}"
        done
    fi

    echo -e "\n${BOLD}How would you like to configure LayoutKeeper?${RESET}"
    echo -e "  1) Use ${GREEN}ALL detected${RESET} terminals"
    echo -e "  2) Select ${YELLOW}ONE specific${RESET} terminal from the list"
    echo -e "  3) Enter a ${CYAN}CUSTOM list${RESET} manually"
    read -p "Choose an option [1/2/3]: " TERM_CHOICE < /dev/tty

    case "$TERM_CHOICE" in
        2)
            read -p "Enter the number of the terminal: " TERM_NUM < /dev/tty
            if [[ "$TERM_NUM" =~ ^[0-9]+$ ]] && [ "$TERM_NUM" -ge 1 ] && [ "$TERM_NUM" -le "${#DETECTED[@]}" ]; then
                SELECTED_TERMS="${DETECTED[$((TERM_NUM-1))]}"
            else
                echo -e "${RED}Invalid selection. Defaulting to all detected.${RESET}"
                SELECTED_TERMS=$(IFS=, ; echo "${DETECTED[*]}")
            fi
            ;;
        3)
            read -p "Enter terminals separated by commas (e.g., kitty,foot): " CUSTOM_TERMS < /dev/tty
            SELECTED_TERMS="${CUSTOM_TERMS// /}"
            ;;
        *)
            SELECTED_TERMS=$(IFS=, ; echo "${DETECTED[*]}")
            ;;
    esac

    echo -e "\n${GREEN}✔ Selected Terminals:${RESET} ${CYAN}$SELECTED_TERMS${RESET}\n"
}

uninstall() {
    echo -e "\n${YELLOW}=> Uninstalling LayoutKeeper...${RESET}"
    systemctl --user stop layoutkeeper.service || true
    systemctl --user disable layoutkeeper.service || true
    rm -f "$SYSTEMD_DIR/layoutkeeper.service"
    systemctl --user daemon-reload
    
    sudo rm -f "$BIN_DIR/layoutkeeper"
    echo -e "${GREEN}=> LayoutKeeper has been uninstalled. (Config files kept in $CONFIG_DIR)${RESET}"
    exit 0
}

if [ "$1" = "--uninstall" ]; then
    uninstall
fi

print_banner
detect_system
print_sysinfo

echo -e "${BOLD}Do you want to proceed with the installation?${RESET}"
read -p " [Y/n]: " CONFIRM < /dev/tty
if [[ "$CONFIRM" =~ ^[Nn]$ ]]; then
    echo -e "${RED}Installation aborted.${RESET}"
    exit 0
fi

select_terminals

echo -e "${BOLD}[Installation]${RESET}"
echo -e "${BLUE}=>${RESET} Installing dependencies using ${YELLOW}$PKG_MNGR${RESET}..."

WAYLAND_DEPS="jq"
X11_DEPS="xdotool"

case "$PKG_MNGR" in
    apt-get)
        sudo apt-get update
        sudo apt-get install -y $WAYLAND_DEPS $X11_DEPS x11-utils xkbcomp setxkbmap
        ;;
    pacman)
        sudo pacman -Sy --noconfirm $WAYLAND_DEPS $X11_DEPS xorg-xprop xorg-setxkbmap xorg-xkbcomp
        ;;
    dnf)
        sudo dnf install -y $WAYLAND_DEPS $X11_DEPS xprop libX11 xkbcomp
        ;;
    zypper)
        sudo zypper install -y $WAYLAND_DEPS $X11_DEPS xprop setxkbmap
        ;;
    *)
        echo -e "${YELLOW}=> Warning: Unsupported package manager. Please ensure 'jq', 'xdotool', 'xprop', and 'setxkbmap' are installed.${RESET}"
        ;;
esac

echo -e "${BLUE}=>${RESET} Installing executable to ${CYAN}$BIN_DIR/layoutkeeper${RESET}"
if [ -f "layoutkeeper" ]; then
    sudo cp layoutkeeper "$BIN_DIR/layoutkeeper"
else
    echo -e "${YELLOW}=> Fetching executable directly from GitHub...${RESET}"
    sudo curl -fsSL "$REPO_RAW_URL/layoutkeeper" -o "$BIN_DIR/layoutkeeper"
fi
sudo chmod +x "$BIN_DIR/layoutkeeper"

echo -e "${BLUE}=>${RESET} Setting up configuration in ${CYAN}$CONFIG_DIR${RESET}"
mkdir -p "$CONFIG_DIR"
cat > "$CONFIG_DIR/config" << EOF
[LayoutKeeper]
enabled = true
debug = false
restore_layout = true
terminals = $SELECTED_TERMS
us_index = 0
restore_index = 1
EOF

echo -e "${BLUE}=>${RESET} Setting up systemd user service"
mkdir -p "$SYSTEMD_DIR"
cat > "$SYSTEMD_DIR/layoutkeeper.service" << EOF
[Unit]
Description=LayoutKeeper - Terminal Keyboard Switcher
After=graphical-session.target

[Service]
Type=simple
ExecStart=$BIN_DIR/layoutkeeper
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable layoutkeeper.service
systemctl --user restart layoutkeeper.service

echo -e "\n${GREEN}${BOLD}✔ Installation Complete!${RESET}"
echo -e "Service is running in the background."
echo -e "Run ${CYAN}layoutkeeper status${RESET} to check the status."
echo -e "Run ${CYAN}layoutkeeper help${RESET} for more commands."