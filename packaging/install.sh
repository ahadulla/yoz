#!/bin/bash
set -e

REPO="ahadulla/yoz"
INSTALL_DIR="/usr/local/bin"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)  TARGET="x86_64-unknown-linux-gnu" ;;
  darwin)
    case "$ARCH" in
      arm64) TARGET="aarch64-apple-darwin" ;;
      *)     TARGET="x86_64-apple-darwin" ;;
    esac
    ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

VERSION=$(curl -sI "https://github.com/$REPO/releases/latest" | grep -i location | sed 's/.*tag\///' | tr -d '\r\n')
if [ -z "$VERSION" ]; then
  echo "Versiyani aniqlab bo'lmadi"
  exit 1
fi

URL="https://github.com/$REPO/releases/download/$VERSION/yoz-$TARGET.tar.gz"
echo "Yuklanmoqda: yoz $VERSION ($TARGET)..."

TMP=$(mktemp -d)
curl -sL "$URL" -o "$TMP/yoz.tar.gz"
tar xzf "$TMP/yoz.tar.gz" -C "$TMP"

sudo install -m 755 "$TMP/yoz" "$INSTALL_DIR/yoz"
rm -rf "$TMP"

echo "yoz $VERSION o'rnatildi: $INSTALL_DIR/yoz"
