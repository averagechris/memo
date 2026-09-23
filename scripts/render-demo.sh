#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
vhs_bin=${VHS_BIN:-$(command -v vhs || true)}

if [[ -z "$vhs_bin" || ! -x "$vhs_bin" ]]; then
  echo "error: VHS_BIN must name an executable, or vhs must be on PATH" >&2
  exit 1
fi

cargo build -q --manifest-path "$repo_root/Cargo.toml"

demo_root=$(mktemp -d "${TMPDIR:-/tmp}/memo-demo.XXXXXX")
cleanup() {
  rm -rf "$demo_root"
}
trap cleanup EXIT

mkdir -p "$demo_root/bin" "$demo_root/home" "$demo_root/xdg-data" "$demo_root/xdg-config"
cp "$repo_root/target/debug/memo" "$demo_root/bin/memo"

export HOME="$demo_root/home"
export XDG_DATA_HOME="$demo_root/xdg-data"
export XDG_CONFIG_HOME="$demo_root/xdg-config"
export MEMO_DATA_DIR="$demo_root/memo-data"
export PATH="$demo_root/bin:$PATH"

cd "$repo_root"
rm -f docs/demo.gif
"$vhs_bin" docs/demo.tape

if [[ ! -s docs/demo.gif ]] || [[ $(LC_ALL=C head -c 6 docs/demo.gif) != "GIF89a" ]]; then
  echo "error: VHS did not produce a GIF89a demo" >&2
  exit 1
fi

if ! command -v ffprobe >/dev/null 2>&1; then
  echo "error: ffprobe is required to verify the rendered demo" >&2
  exit 1
fi

dimensions=$(ffprobe -v error -select_streams v:0 \
  -show_entries stream=width,height -of csv=s=x:p=0 docs/demo.gif)
frames=$(ffprobe -v error -select_streams v:0 \
  -show_entries stream=nb_frames -of csv=p=0 docs/demo.gif)
if [[ ! "$dimensions" =~ ^[1-9][0-9]*x[1-9][0-9]*$ ]] ||
  [[ ! "$frames" =~ ^[0-9]+$ ]] || ((frames < 2)); then
  echo "error: invalid demo media: dimensions=$dimensions frames=$frames" >&2
  exit 1
fi

echo "rendered docs/demo.gif ($dimensions, $frames frames)"
