#!/bin/bash

CONFIG_DIR="$HOME/.config/layoutkeeper"
CONFIG_FILE="$CONFIG_DIR/config"

ENV_TYPE=""
DE_WM=""
LAYOUT_SAVED=false
SAVED_LAYOUT=""

US_INDEX="0"
RESTORE_INDEX="1"

load_config() {
    if [ ! -f "$CONFIG_FILE" ]; then
        echo "Error: Config file missing. Please reinstall or run setup."
        exit 1
    fi
    TERMINALS=$(grep -E "^terminals" "$CONFIG_FILE" | cut -d'=' -f2 | tr -d ' ' | tr ',' ' ')
    DEBUG=$(grep -E "^debug" "$CONFIG_FILE" | cut -d'=' -f2 | tr -d ' ' | tr '[:upper:]' '[:lower:]')
    
    local cfg_us=$(grep -E "^us_index" "$CONFIG_FILE" | cut -d'=' -f2 | tr -d ' ')
    local cfg_restore=$(grep -E "^restore_index" "$CONFIG_FILE" | cut -d'=' -f2 | tr -d ' ')
    
    if [ -n "$cfg_us" ]; then US_INDEX="$cfg_us"; fi
    if [ -n "$cfg_restore" ]; then RESTORE_INDEX="$cfg_restore"; fi
}

save_config() {
    local term_list=$(echo "$TERMINALS" | tr ' ' ',')
    sed -i "s/^terminals.*/terminals = $term_list/" "$CONFIG_FILE"
}

log() {
    if [ "$DEBUG" = "true" ]; then
        echo "[LayoutKeeper] $1"
    fi
}

detect_env() {
    if [ -n "$WAYLAND_DISPLAY" ]; then
        ENV_TYPE="wayland"
        if [ -n "$HYPRLAND_INSTANCE_SIGNATURE" ]; then
            DE_WM="hyprland"
        elif [ -n "$SWAYSOCK" ]; then
            DE_WM="sway"
        elif [ -n "$NIRI_SOCKET" ]; then
            DE_WM="niri"
        elif [[ "${XDG_CURRENT_DESKTOP,,}" == *"gnome"* ]]; then
            DE_WM="gnome"
        elif [[ "${XDG_CURRENT_DESKTOP,,}" == *"kde"* ]]; then
            DE_WM="kde"
        else
            DE_WM="generic-wayland"
        fi
    elif [ -n "$DISPLAY" ]; then
        ENV_TYPE="x11"
        DE_WM="x11"
    else
        echo "Error: No display server detected."
        exit 1
    fi
    log "Environment detected: $ENV_TYPE ($DE_WM)"
}

get_current_layout() {
    if [ "$ENV_TYPE" = "x11" ]; then
        setxkbmap -query 2>/dev/null | grep layout | awk '{print $2}'
    elif [ "$ENV_TYPE" = "wayland" ]; then
        case "$DE_WM" in
            hyprland)
                hyprctl devices -j 2>/dev/null | jq -r '.keyboards[] | select(.main == true) | .active_keymap'
                ;;
            sway)
                swaymsg -t get_inputs 2>/dev/null | jq -r '.[] | select(.type=="keyboard") | .xkb_active_layout_name' | head -n1
                ;;
            niri)
                local niri_idx=$(niri msg -j keyboard-layouts 2>/dev/null | jq -r '.current_idx // empty')
                if [ -z "$niri_idx" ] || [ "$niri_idx" = "null" ]; then
                    niri_idx=$(niri msg keyboard-layouts 2>/dev/null | grep -E '^[ \*]' | awk '{if ($1 == "*") print NR-1}' | head -n1)
                fi
                echo "$niri_idx"
                ;;
            gnome)
                gsettings get org.gnome.desktop.input-sources current 2>/dev/null | awk '{print $2}'
                ;;
            kde)
                qdbus org.kde.keyboard /Layouts org.kde.KeyboardLayouts.getLayouts 2>/dev/null | head -n1
                ;;
        esac
    fi
}

set_layout_us() {
    log "Switching to US layout"
    if [ "$ENV_TYPE" = "x11" ]; then
        setxkbmap us 2>/dev/null
    elif [ "$ENV_TYPE" = "wayland" ]; then
        case "$DE_WM" in
            hyprland)
                local kbds=$(hyprctl devices -j 2>/dev/null | jq -r '.keyboards[].name')
                for k in $kbds; do
                    hyprctl switchxkblayout "$k" "$US_INDEX" >/dev/null 2>&1
                done
                ;;
            sway)
                swaymsg input type:keyboard xkb_layout us >/dev/null 2>&1
                ;;
            niri)
                niri msg action switch-layout "$US_INDEX" >/dev/null 2>&1
                ;;
            gnome)
                gsettings set org.gnome.desktop.input-sources current 0 >/dev/null 2>&1
                ;;
            kde)
                qdbus org.kde.keyboard /Layouts org.kde.KeyboardLayouts.setLayout "us" >/dev/null 2>&1
                ;;
        esac
    fi
}

restore_layout() {
    if [ -z "$SAVED_LAYOUT" ]; then return; fi
    log "Restoring layout to: $SAVED_LAYOUT"
    
    if [ "$ENV_TYPE" = "x11" ]; then
        setxkbmap "$SAVED_LAYOUT" 2>/dev/null
    elif [ "$ENV_TYPE" = "wayland" ]; then
        case "$DE_WM" in
            hyprland)
                local kbds=$(hyprctl devices -j 2>/dev/null | jq -r '.keyboards[].name')
                for k in $kbds; do
                    hyprctl switchxkblayout "$k" "$SAVED_LAYOUT" >/dev/null 2>&1
                done
                ;;
            sway)
                swaymsg input type:keyboard xkb_layout "$SAVED_LAYOUT" >/dev/null 2>&1
                ;;
            niri)
                niri msg action switch-layout "$SAVED_LAYOUT" >/dev/null 2>&1
                ;;
            gnome)
                gsettings set org.gnome.desktop.input-sources current "$SAVED_LAYOUT" >/dev/null 2>&1
                ;;
            kde)
                qdbus org.kde.keyboard /Layouts org.kde.KeyboardLayouts.setLayout "$SAVED_LAYOUT" >/dev/null 2>&1
                ;;
        esac
    fi
}

get_focused_window() {
    if [ "$ENV_TYPE" = "x11" ]; then
        local win_id=$(xdotool getwindowfocus 2>/dev/null)
        if [ -n "$win_id" ]; then
            xprop -id "$win_id" WM_CLASS 2>/dev/null | awk -F'"' '{print $4}' | tr '[:upper:]' '[:lower:]'
        fi
    elif [ "$ENV_TYPE" = "wayland" ]; then
        case "$DE_WM" in
            hyprland)
                hyprctl activewindow -j 2>/dev/null | jq -r '.class // empty' | tr '[:upper:]' '[:lower:]'
                ;;
            sway)
                swaymsg -t get_tree 2>/dev/null | jq -r '.. | select(.focused? == true) | .app_id // empty' | tr '[:upper:]' '[:lower:]'
                ;;
            niri)
                niri msg -j focused-window 2>/dev/null | jq -r '.["app-id"] // .app_id // .title // empty' | tr '[:upper:]' '[:lower:]'
                ;;
            gnome)
                gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell --method org.gnome.Shell.Eval "global.display.focus_window.get_wm_class()" 2>/dev/null | awk -F'"' '{print $2}' | tr '[:upper:]' '[:lower:]'
                ;;
            *)
                echo "unknown"
                ;;
        esac
    fi
}

is_terminal() {
    local class="$1"
    if [ -z "$class" ] || [ "$class" = "null" ]; then return 1; fi
    
    for term in $TERMINALS; do
        if [[ "$class" == *"$term"* ]]; then
            return 0
        fi
    done
    return 1
}

monitor_focus() {
    log "Starting focus monitor loop..."
    local PREV_WINDOW=""
    
    while true; do
        CURRENT_WINDOW=$(get_focused_window)
        
        if [ "$CURRENT_WINDOW" != "$PREV_WINDOW" ]; then
            log "Focus changed: $CURRENT_WINDOW"
            
            if is_terminal "$CURRENT_WINDOW"; then
                if [ "$LAYOUT_SAVED" = false ]; then
                    SAVED_LAYOUT=$(get_current_layout)
                    
                    if [ -z "$SAVED_LAYOUT" ]; then
                        SAVED_LAYOUT="$RESTORE_INDEX"
                    fi

                    log "Captured layout before switching: $SAVED_LAYOUT"
                    
                    if [ "$SAVED_LAYOUT" != "$US_INDEX" ] && [ "$SAVED_LAYOUT" != "us" ]; then
                        set_layout_us
                    else
                        log "Already in US layout, no switch required."
                    fi
                    
                    LAYOUT_SAVED=true
                fi
            else
                if [ "$LAYOUT_SAVED" = true ]; then
                    restore_layout
                    LAYOUT_SAVED=false
                    log "Left Terminal. Restored layout to $SAVED_LAYOUT."
                fi
            fi
            PREV_WINDOW="$CURRENT_WINDOW"
        fi
        
        sleep 0.1
    done
}

show_help() {
    echo "LayoutKeeper - Smart Terminal Keyboard Switcher"
    echo "Usage: layoutkeeper [command]"
    echo ""
    echo "Commands:"
    echo "  on              Start and enable the service"
    echo "  off             Stop and disable the service"
    echo "  status          Show service status"
    echo "  restart         Restart the service"
    echo "  list            List monitored terminals"
    echo "  add <term>      Add a terminal to monitor"
    echo "  remove <term>   Remove a terminal"
    echo "  logs            Show service logs"
    echo "  help            Show this help message"
}

manage_terminals() {
    local action="$1"
    local term="$2"
    
    if [ -z "$term" ] && [ "$action" != "list" ]; then
        echo "Error: Missing terminal name."
        exit 1
    fi

    case "$action" in
        list)
            echo "Monitored terminals:"
            for t in $TERMINALS; do echo "  - $t"; done
            ;;
        add)
            if [[ " $TERMINALS " =~ " $term " ]]; then
                echo "Terminal '$term' is already monitored."
            else
                TERMINALS="$TERMINALS $term"
                save_config
                echo "Added '$term'."
                systemctl --user restart layoutkeeper
            fi
            ;;
        remove)
            if [[ " $TERMINALS " =~ " $term " ]]; then
                TERMINALS=$(echo "$TERMINALS" | sed "s/\b$term\b//g" | tr -s ' ' | sed 's/^ //;s/ $//')
                save_config
                echo "Removed '$term'."
                systemctl --user restart layoutkeeper
            else
                echo "Terminal '$term' not found in the list."
            fi
            ;;
    esac
}

load_config

if [ -z "$1" ]; then
    detect_env
    monitor_focus
    exit 0
fi

case "$1" in
    on)
        systemctl --user enable --now layoutkeeper
        echo "LayoutKeeper service enabled and started."
        ;;
    off)
        systemctl --user disable --now layoutkeeper
        echo "LayoutKeeper service disabled and stopped."
        ;;
    status)
        systemctl --user status layoutkeeper
        ;;
    restart)
        systemctl --user restart layoutkeeper
        echo "LayoutKeeper service restarted."
        ;;
    logs)
        journalctl --user -u layoutkeeper -f
        ;;
    list|add|remove)
        manage_terminals "$1" "$2"
        ;;
    help|*)
        show_help
        ;;
esac
