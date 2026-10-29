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
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
out="$repo/docs/screenshots"
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

cargo build --quiet --manifest-path "$repo/Cargo.toml"
XDG_CONFIG_HOME="$demo/config" XDG_DATA_HOME="$demo/data" VOLTKIT_HOSTS_FILE="$demo/hosts" \
    "$repo/target/debug/voltkit" screenshots "$out"
