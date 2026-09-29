#!/bin/sh
set -eu

# select an available distribution for this machine
system=$(uname -s)
architecture=$(uname -m)
if [ "$system" = Darwin ] && [ "$(/usr/sbin/sysctl -in hw.optional.arm64 2>/dev/null || true)" = 1 ]; then
    architecture=arm64
fi
case "$system/$architecture" in
    Darwin/arm64) target=aarch64-apple-darwin ;;
    Darwin/x86_64) target=x86_64-apple-darwin ;;
    Linux/aarch64) target=aarch64-unknown-linux-gnu ;;
    Linux/x86_64) target=x86_64-unknown-linux-gnu ;;
    *) printf '%s\n' 'Unsupported platform.' >&2; exit 1 ;;
esac
# select the complete application archive for per-user installation
format=tar.gz
case "$target/$format" in
# __DISTRIBUTIONS__
    *) printf '%s\n' 'A release is not available for this platform.' >&2; exit 1 ;;
esac

# download into a private directory and verify the published digest before execution
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM
artifact="$temporary/destack.$format"
curl --fail --location --proto '=https' --proto-redir '=https' --tlsv1.2 "$url" -o "$artifact"
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$artifact" | cut -d ' ' -f 1)
else
    actual=$(shasum -a 256 "$artifact" | cut -d ' ' -f 1)
fi
if [ "$actual" != "$sha256" ]; then
    printf '%s\n' 'Download digest does not match.' >&2
    exit 1
fi
tar -xzf "$artifact" -C "$temporary"

# verify the notarized application before executing its bundled installer
if [ "$system" = Darwin ]; then
    codesign --verify --deep --strict "$temporary/Destack.app"
    spctl --assess --type execute "$temporary/Destack.app"
fi

# let the compiled verifier authenticate and install the complete signed release
"$temporary/bin/destack" install --archive "$temporary/destack.tar.gz"
