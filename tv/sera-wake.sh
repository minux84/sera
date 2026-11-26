#!/bin/sh
# Sera on wake. Quick Start+ resumes the TV without booting, so
# 70-sera-autostart never runs after the power button. This watches the
# power state and opens Sera each time the TV comes back to Active.
# LG system apps are not changed.
# Installed and switched by Sera Settings › System › Root features.
# Disable: touch /var/lib/webosbrew/sera-autostart-disabled
if [ -z "$SERA_WAKE_CHILD" ]; then
  SERA_WAKE_CHILD=1 nohup "$0" > /tmp/sera-wake.log 2>&1 < /dev/null &
  exit 0
fi
pidfile=/tmp/sera-wake.pid
if [ -f "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
  exit 0
fi
echo $$ > "$pidfile"
app=/media/developer/apps/usr/palm/applications/it.bytebuilder.sera/appinfo.json

# $1: attempts Sera must reach in the foreground before stopping (default 3).
# $2: maximum attempts (default 15).
open_sera() {
  [ -e /var/lib/webosbrew/sera-autostart-disabled ] && return
  [ -f "$app" ] || return
  hold=${1:-3}
  attempt=0
  while [ "$attempt" -lt "${2:-15}" ]; do
    attempt=$((attempt + 1))
    # Launch first: right after resume the foreground report can be stale.
    reply=$(luna-send -n 1 luna://com.webos.service.applicationManager/launch '{"id":"it.bytebuilder.sera"}')
    echo "$(date '+%T') wake attempt $attempt: $reply"
    sleep 1
    active=$(luna-send -n 1 luna://com.webos.applicationManager/getForegroundAppInfo '{}')
    echo "$(date '+%T') foreground: $active"
    case "$active" in
      *'"appId":"it.bytebuilder.sera"'*) [ "$attempt" -ge "$hold" ] && return ;;
    esac
  done
}

# Resume reports "Prepare Resume", "LastInput Ready", "Screen On" (onOff on)
# about four seconds before "Active"; open Sera at the first of them.
# A cold boot never reports onOff: the first reply is already "Active".
# Then open Sera once, and hold it longer, because the TV restores the
# last input some seconds after boot and would cover it.
# /tmp is emptied at boot, so the marker keeps this to once per boot.
booted=/tmp/sera-wake.booted
while true; do
  armed=1
  luna-send -i luna://com.webos.service.tvpower/power/getPowerState '{"subscribe":true}' |
    while read -r line; do
      echo "$(date '+%T') power: $line"
      line=$(echo "$line" | tr -d ' ')
      if [ ! -e "$booted" ]; then
        : > "$booted"
        uptime=$(cut -d. -f1 /proc/uptime)
        case "$line" in
          *'"state":"Active"'*)
            if [ "${uptime:-999}" -lt 300 ]; then
              echo "$(date '+%T') boot (uptime ${uptime}s): opening Sera"
              armed=0
              open_sera 20 40 &
            fi
            ;;
        esac
      fi
      case "$line" in
        *'"onOff":"off"'*) armed=1 ;;
        *'"onOff":"on"'*)
          if [ "$armed" = 1 ]; then
            armed=0
            open_sera &
          fi
          ;;
      esac
    done
  sleep 2
done
