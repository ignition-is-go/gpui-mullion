#!/usr/bin/env bash
set -euo pipefail

runtime_dir="$(mktemp -d)"
weston_log="${TMPDIR:-/tmp}/mullion-weston.log"
xvfb_log="${TMPDIR:-/tmp}/mullion-xvfb.log"
demo_log="${TMPDIR:-/tmp}/mullion-wayland-demo.log"
chmod 700 "$runtime_dir"
export XDG_RUNTIME_DIR="$runtime_dir"
export WAYLAND_DISPLAY=wayland-mullion-ci

# Weston headless publishes no wl_seat, while GPUI requires one. Nest Weston's
# X11 backend under Xvfb so Mullion still exercises a real Wayland client,
# surface, and input path with a deterministic virtual seat.
Xvfb :99 -screen 0 1280x720x24 -nolisten tcp >"$xvfb_log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 1 100); do
  [[ -S /tmp/.X11-unix/X99 ]] && break
  kill -0 "$xvfb_pid" 2>/dev/null || { cat "$xvfb_log"; exit 1; }
  sleep 0.1
done
[[ -S /tmp/.X11-unix/X99 ]] || { cat "$xvfb_log"; exit 1; }

DISPLAY=:99 weston --backend=x11-backend.so --socket="$WAYLAND_DISPLAY" --idle-time=0 --use-pixman --width=1280 --height=720 >"$weston_log" 2>&1 &
weston_pid=$!
cleanup() {
  kill "$weston_pid" "$xvfb_pid" 2>/dev/null || true
  wait "$weston_pid" 2>/dev/null || true
  wait "$xvfb_pid" 2>/dev/null || true
  rm -rf "$runtime_dir"
}
trap cleanup EXIT
unset DISPLAY

for _ in $(seq 1 100); do
  if [[ -S "$runtime_dir/$WAYLAND_DISPLAY" ]]; then
    break
  fi
  if ! kill -0 "$weston_pid" 2>/dev/null; then
    cat "$weston_log"
    exit 1
  fi
  sleep 0.1
done
[[ -S "$runtime_dir/$WAYLAND_DISPLAY" ]] || { cat "$weston_log"; exit 1; }

set +e
LIBGL_ALWAYS_SOFTWARE=1 timeout --signal=TERM 15s target/debug/examples/demo >"$demo_log" 2>&1
status=$?
set -e
if [[ $status -ne 124 ]]; then
  cat "$weston_log"
  cat "$demo_log"
  echo "demo exited before the Wayland liveness window (status $status)" >&2
  exit 1
fi
if grep -Fq "panicked at" "$demo_log"; then
  cat "$demo_log"
  exit 1
fi
cat "$demo_log"
echo "Mullion demo remained live on native Wayland for 15 seconds"
