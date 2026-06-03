#!/bin/sh
set -e

REPO="babanin/pulsar"
INSTALL_DIR="/opt/pulsar"
BIN_DIR="/usr/local/bin"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS-$ARCH" in
    darwin-arm64)  PKG="pulsar-macos-aarch64.tar.gz" ;;
    darwin-x86_64) PKG="pulsar-macos-x86_64.tar.gz" ;;
    linux-x86_64)  PKG="pulsar-linux-x86_64.tar.gz" ;;
    *) echo "Unsupported: $OS-$ARCH"; exit 1 ;;
esac

if [ "$(id -u)" -ne 0 ]; then
    echo "Root required. Run with sudo."
    exit 1
fi

echo "Downloading Pulsar for $OS-$ARCH ..."
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

[ -z "$LATEST" ] && echo "Failed to detect latest version" && exit 1

rm -rf /tmp/pulsar-install
mkdir -p /tmp/pulsar-install
cd /tmp/pulsar-install

curl -fsSL "https://github.com/$REPO/releases/download/$LATEST/$PKG" | tar xz

rm -rf "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
cp -R pulsar/* "$INSTALL_DIR/"

rm -f "$BIN_DIR/pulsar"
mkdir -p "$BIN_DIR"
ln -sf "$INSTALL_DIR/pulsar" "$BIN_DIR/pulsar"

rm -rf /tmp/pulsar-install

echo "Pulsar $LATEST installed"
echo "  $INSTALL_DIR/pulsar"
echo "  $BIN_DIR/pulsar -> $INSTALL_DIR/pulsar"
