#!/bin/sh
# ion installer — detects platform and downloads the correct binary
# Usage: curl -sSf https://ion.cybergenii.com/install.sh | sh

set -e

REPO="cybergenii/ion"
BINARY="ion"
INSTALL_DIR="${ION_INSTALL_DIR:-/usr/local/bin}"

# ── Detect OS ─────────────────────────────────────────────────────────────────
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
  linux)   OS="linux" ;;
  darwin)  OS="macos" ;;
  mingw*|msys*|cygwin*) OS="windows" ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

# ── Detect Architecture ───────────────────────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64)   ARCH="x86_64" ;;
  aarch64|arm64)  ARCH="aarch64" ;;
  armv7*)         ARCH="armv7" ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

# ── Build artifact name ───────────────────────────────────────────────────────
if [ "$OS" = "windows" ]; then
  ARTIFACT="ion-${OS}-${ARCH}.exe"
  EXT=".zip"
else
  ARTIFACT="ion-${OS}-${ARCH}"
  EXT=".tar.gz"
fi

# ── Fetch latest release tag ──────────────────────────────────────────────────
echo "→ Fetching latest ion release..."
TAG=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$TAG" ]; then
  echo "Failed to fetch latest release tag."
  exit 1
fi

echo "→ Latest version: $TAG"
echo "→ Platform: $OS/$ARCH"

# ── Download ──────────────────────────────────────────────────────────────────
URL="https://github.com/${REPO}/releases/download/${TAG}/${ARTIFACT}${EXT}"
TMP=$(mktemp -d)

echo "→ Downloading from: $URL"
curl -sSfL "$URL" -o "$TMP/ion_archive${EXT}"

# ── Extract ───────────────────────────────────────────────────────────────────
cd "$TMP"
if [ "$EXT" = ".tar.gz" ]; then
  tar xzf "ion_archive${EXT}"
else
  unzip -q "ion_archive${EXT}"
fi

# ── Install ───────────────────────────────────────────────────────────────────
chmod +x "$ARTIFACT"

if [ -w "$INSTALL_DIR" ]; then
  mv "$ARTIFACT" "$INSTALL_DIR/$BINARY"
else
  echo "→ Requesting sudo to install to $INSTALL_DIR..."
  sudo mv "$ARTIFACT" "$INSTALL_DIR/$BINARY"
fi

# ── Verify ────────────────────────────────────────────────────────────────────
echo ""
echo "✅ ion installed successfully!"
echo ""
ion --version
echo ""
echo "Run 'ion --help' to get started."