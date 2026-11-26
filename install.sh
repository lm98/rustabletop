#!/bin/sh
set -eu

repo="lm98/rustabletop"
bin_dir="${RUSTABLETOP_INSTALL_DIR:-$HOME/.local/bin}"
fallback="cargo install --git https://github.com/$repo rustabletop"

fail() {
  echo "error: $1" >&2
  echo "fallback (builds from source, ~8 minutes): $fallback" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"

archive="tar.gz"
exe=""
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64 | Linux-amd64) target="x86_64-unknown-linux-gnu" ;;
  Darwin-arm64 | Darwin-aarch64) target="aarch64-apple-darwin" ;;
  MINGW*-x86_64 | MSYS*-x86_64 | CYGWIN*-x86_64)
    target="x86_64-pc-windows-msvc"
    archive="zip"
    exe=".exe"
    ;;
  *) fail "no prebuilt rustabletop for $(uname -s) $(uname -m)" ;;
esac

tag="${RUSTABLETOP_VERSION:-}"
case "$tag" in "" | v*) ;; *) tag="v$tag" ;; esac
if [ -z "$tag" ]; then
  tag=$(curl -fsSL "https://api.github.com/repos/$repo/releases?per_page=1" 2>/dev/null \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1) || true
  [ -n "$tag" ] || fail "no release of $repo found"
fi

url="https://github.com/$repo/releases/download/$tag/rustabletop-$target.$archive"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "$url" -o "$tmp/rustabletop.$archive" || fail "cannot download $url"
if [ "$archive" = "zip" ]; then
  unzip -q "$tmp/rustabletop.zip" -d "$tmp" || fail "cannot extract $url (unzip is required)"
else
  tar -xzf "$tmp/rustabletop.tar.gz" -C "$tmp"
fi
mkdir -p "$bin_dir"
cp "$tmp/rustabletop$exe" "$bin_dir/rustabletop$exe"
chmod 755 "$bin_dir/rustabletop$exe"

echo "installed rustabletop $tag in $bin_dir"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) echo "add it to PATH: export PATH=\"$bin_dir:\$PATH\"" ;;
esac
"$bin_dir/rustabletop$exe" --version
