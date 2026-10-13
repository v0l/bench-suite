#!/bin/sh
set -eu

repo=v0l/bench-suite
bin=${BENCH_SUITE_BIN:-$HOME/.local/bin}

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) target=x86_64-unknown-linux-gnu ;;
  Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-gnu ;;
  Darwin-arm64) target=aarch64-apple-darwin ;;
  Darwin-x86_64) target=x86_64-apple-darwin ;;
  *) echo "no bench suite build for $(uname -s) $(uname -m); build it from source" >&2; exit 1 ;;
esac

name=bench-suite-$target
url=https://github.com/$repo/releases/latest/download/$name.tar.gz
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "downloading $url"
curl -fsSL "$url" -o "$tmp/$name.tar.gz"
curl -fsSL "$url.sha256" -o "$tmp/$name.tar.gz.sha256"
(cd "$tmp" && if command -v sha256sum >/dev/null; then sha256sum -c "$name.tar.gz.sha256"; else shasum -a 256 -c "$name.tar.gz.sha256"; fi) >/dev/null
tar xzf "$tmp/$name.tar.gz" -C "$tmp"

mkdir -p "$bin"
install -m 755 "$tmp/$name/bench-suite" "$bin/bench-suite"

if [ "$(uname -s)" = Linux ]; then
  icons=${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps
  apps=${XDG_DATA_HOME:-$HOME/.local/share}/applications
  mkdir -p "$icons" "$apps"
  curl -fsSL "https://raw.githubusercontent.com/$repo/master/assets/icons/bench-suite-256.png" -o "$icons/bench-suite.png" || true
  cat > "$apps/bench-suite.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Bench Suite
Comment=Install, update and start the bench tools
Exec=$bin/bench-suite
Icon=bench-suite
Terminal=false
Categories=Development;Engineering;Electronics;
StartupWMClass=bench-suite
EOF
fi

echo "installed $bin/bench-suite"
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "add $bin to your PATH" ;;
esac
