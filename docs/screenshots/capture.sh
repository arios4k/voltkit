#!/usr/bin/env bash
# Regenerates the README screenshots from the real app.
#
#   docs/screenshots/capture.sh
#
# A Voltkit window opens on your desktop for about ten seconds and steps
# through its pages in light and dark; leave it alone until it closes. It
# uses a throwaway demo config and color history, so nothing personal ends up
# in the images.
#
# On distributions that theme libadwaita, name the matching light and dark
# themes so both passes look consistent, e.g. on Zorin OS:
#   VOLTKIT_LIGHT_THEME=ZorinBlue-Light VOLTKIT_DARK_THEME=ZorinBlue-Dark docs/screenshots/capture.sh
#
# Pass a language to capture the translated app into docs/screenshots/LANG,
# for README.LANG.md:
#   docs/screenshots/capture.sh es
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
lang=${1:-en}
case $lang in
    en) out="$repo/docs/screenshots"; locale=en_US ;;
    es) out="$repo/docs/screenshots/es"; locale=es_MX ;;
    de) out="$repo/docs/screenshots/de"; locale=de_DE ;;
    fr) out="$repo/docs/screenshots/fr"; locale=fr_FR ;;
    zh_CN) out="$repo/docs/screenshots/zh_CN"; locale=zh_CN ;;
    *) echo "unknown language $lang (en, es, de, fr or zh_CN)" >&2; exit 2 ;;
esac
mkdir -p "$out"
demo=$(mktemp -d)
trap 'rm -rf "$demo"' EXIT

mkdir -p "$demo/config/voltkit" "$demo/data/voltkit"
cat > "$demo/config/voltkit/config.toml" <<'TOML'
[[snippets]]
trigger = ";date"
template = "{date:%Y-%m-%d}"

[[snippets]]
trigger = ";now"
template = "{date:%A, %B %d} at {time}"

[[snippets]]
trigger = ";sig"
template = "Best regards,\nAlexis Rios-Negron"

[[snippets]]
trigger = ";meet"
template = "Meeting notes, {date:%B %d}\nAttendees: {arg:attendees}\nAgenda: {arg:topic}"

[[snippets]]
trigger = ";quote"
template = "> {clipboard}\n\n— {arg:author}"

[[snippets]]
trigger = ";shrug"
template = "¯\\_(ツ)_/¯"
TOML
printf '%s\n' '#7C3AED' '#4F46E5' '#F97316' '#FACC15' '#10B981' '#0EA5E9' '#EC4899' '#EF4444' \
    > "$demo/data/voltkit/colors.txt"
cat > "$demo/hosts" <<'HOSTS'
127.0.0.1	localhost
127.0.1.1	workstation
::1	ip6-localhost ip6-loopback
127.0.0.1	myapp.test api.myapp.test	# local development
192.168.1.20	nas.home	# storage box
# 10.0.0.5	staging.example.com
HOSTS
mkdir -p "$demo/config/environment.d"
printf '%s\n' '# Environment variables set in Voltkit. Applied at login.' 'EDITOR=nano' 'PATH=${PATH}:${HOME}/.local/bin' \
    'GOPATH=${HOME}/go' > "$demo/config/environment.d/90-voltkit.conf"

# Remembered app permissions: table, id, app, values (tab-separated).
printf '%s\t%s\t%s\t%s\n' \
    screenshot screenshot com.obsproject.Studio yes \
    screencast screencast com.obsproject.Studio '' \
    background background com.obsproject.Studio no \
    devices camera us.zoom.Zoom yes \
    devices microphone us.zoom.Zoom yes \
    screenshot screenshot us.zoom.Zoom no \
    devices camera org.gnome.Snapshot yes \
    devices microphone org.gnome.Snapshot no \
    screenshot screenshot io.github.arios4k.Voltkit yes \
    > "$demo/permissions"

cargo build --quiet --manifest-path "$repo/Cargo.toml"
# A clean environment, so the "Already in Your Session" list shows typical
# values instead of the real ones (which can include secrets).
env -i \
    HOME="$HOME" WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-}" DISPLAY="${DISPLAY:-}" \
    XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" DBUS_SESSION_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS" \
    XDG_CURRENT_DESKTOP="${XDG_CURRENT_DESKTOP:-GNOME}" XDG_DATA_DIRS="${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" \
    PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    LANG=$locale.UTF-8 LANGUAGE=$locale:$lang LC_TIME=$locale.UTF-8 \
    BROWSER=firefox JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64 \
    ${VOLTKIT_LIGHT_THEME:+VOLTKIT_LIGHT_THEME="$VOLTKIT_LIGHT_THEME"} \
    ${VOLTKIT_DARK_THEME:+VOLTKIT_DARK_THEME="$VOLTKIT_DARK_THEME"} \
    XDG_CONFIG_HOME="$demo/config" XDG_DATA_HOME="$demo/data" VOLTKIT_HOSTS_FILE="$demo/hosts" \
    VOLTKIT_PERMISSIONS_FILE="$demo/permissions" VOLTKIT_USAGE_DEMO=1 VOLTKIT_SLEEP_DEMO=1 VOLTKIT_REPORT_DEMO=1 VOLTKIT_SHORTCUTS_DEMO=1 \
    "$repo/target/debug/voltkit" screenshots "$out"
