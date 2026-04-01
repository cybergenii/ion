#!/bin/sh
# ion installer - detects platform and installs the right binary
# Usage: curl -fsSL https://ion.cybergenii.com/install.sh | sh

set -eu

APP_NAME="ion"
REPO="cybergenii/ion"
INSTALL_DIR="${ION_INSTALL_DIR:-${XDG_BIN_HOME:-$HOME/.local/bin}}"
NO_MODIFY_PATH="${ION_NO_MODIFY_PATH:-0}"
PRINT_QUIET="${ION_PRINT_QUIET:-0}"
PRINT_VERBOSE="${ION_PRINT_VERBOSE:-0}"
DOWNLOAD_URL_OVERRIDE="${ION_DOWNLOAD_URL:-}"
VERSION_OVERRIDE="${ION_VERSION:-}"

say() {
  if [ "$PRINT_QUIET" != "1" ]; then
    printf "%s\n" "$1"
  fi
}

say_verbose() {
  if [ "$PRINT_VERBOSE" = "1" ]; then
    printf "%s\n" "$1"
  fi
}

err() {
  printf "ERROR: %s\n" "$1" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || err "need '$1' (command not found)"
}

downloader() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$1" -O "$2"
  else
    err "need 'curl' or 'wget' (command not found)"
  fi
}

detect_os() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "$os" in
    linux) echo "linux" ;;
    darwin) echo "macos" ;;
    mingw*|msys*|cygwin*) echo "windows" ;;
    *) err "unsupported OS: $os" ;;
  esac
}

detect_arch() {
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    armv7*) echo "armv7" ;;
    *) err "unsupported architecture: $arch" ;;
  esac
}

get_latest_tag() {
  need_cmd sed
  need_cmd tr
  api="https://api.github.com/repos/${REPO}/releases/latest"
  tmp_json="$1/latest.json"
  downloader "$api" "$tmp_json"
  sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' "$tmp_json" | tr -d '\r' | sed -n '1p'
}

add_path_hint() {
  case ":$PATH:" in
    *:"$INSTALL_DIR":*)
      return 0
      ;;
  esac

  if [ "$NO_MODIFY_PATH" = "1" ]; then
    say "Add to PATH manually:"
    say "  export PATH=\"$INSTALL_DIR:\$PATH\""
    return 0
  fi

  line="export PATH=\"$INSTALL_DIR:\$PATH\""
  for rc in "$HOME/.profile" "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [ -f "$rc" ]; then
      if ! grep -F "$line" "$rc" >/dev/null 2>&1; then
        printf "\n%s\n" "$line" >> "$rc"
        say "Updated PATH in $rc"
      fi
      return 0
    fi
  done

  printf "\n%s\n" "$line" >> "$HOME/.profile"
  say "Updated PATH in $HOME/.profile"
}

cleanup() {
  if [ -n "${TMP_DIR:-}" ] && [ -d "${TMP_DIR:-}" ]; then
    rm -rf "$TMP_DIR"
  fi
}

main() {
  need_cmd uname
  need_cmd mktemp
  need_cmd chmod
  need_cmd mkdir
  need_cmd mv
  need_cmd tar

  OS="$(detect_os)"
  ARCH="$(detect_arch)"

  if [ "$OS" = "windows" ]; then
    ARTIFACT="${APP_NAME}-${OS}-${ARCH}.exe"
    EXT=".zip"
    need_cmd unzip
  else
    ARTIFACT="${APP_NAME}-${OS}-${ARCH}"
    EXT=".tar.gz"
  fi

  TMP_DIR="$(mktemp -d)"
  trap cleanup EXIT INT TERM

  if [ -n "$VERSION_OVERRIDE" ]; then
    TAG="$VERSION_OVERRIDE"
    case "$TAG" in
      v*) ;;
      *) TAG="v$TAG" ;;
    esac
  else
    say "-> Fetching latest ${APP_NAME} release..."
    TAG="$(get_latest_tag "$TMP_DIR")"
    [ -n "$TAG" ] || err "failed to fetch latest release tag"
  fi

  if [ -n "$DOWNLOAD_URL_OVERRIDE" ]; then
    URL="${DOWNLOAD_URL_OVERRIDE%/}/${ARTIFACT}${EXT}"
  else
    URL="https://github.com/${REPO}/releases/download/${TAG}/${ARTIFACT}${EXT}"
  fi

  say "-> Version: $TAG"
  say "-> Platform: $OS/$ARCH"
  say_verbose "-> Artifact: ${ARTIFACT}${EXT}"
  say "-> Downloading: $URL"
  downloader "$URL" "$TMP_DIR/ion_archive${EXT}"

  if [ "$EXT" = ".tar.gz" ]; then
    tar xzf "$TMP_DIR/ion_archive${EXT}" -C "$TMP_DIR"
  else
    unzip -q "$TMP_DIR/ion_archive${EXT}" -d "$TMP_DIR"
  fi

  [ -f "$TMP_DIR/$ARTIFACT" ] || err "downloaded archive did not contain $ARTIFACT"
  chmod +x "$TMP_DIR/$ARTIFACT"

  mkdir -p "$INSTALL_DIR"
  mv "$TMP_DIR/$ARTIFACT" "$INSTALL_DIR/$APP_NAME"

  say ""
  say "Installed ${APP_NAME} to: $INSTALL_DIR/$APP_NAME"
  add_path_hint
  say ""
  if "$INSTALL_DIR/$APP_NAME" --version >/dev/null 2>&1; then
    "$INSTALL_DIR/$APP_NAME" --version
  else
    say "Run '$INSTALL_DIR/$APP_NAME --version' to verify installation."
  fi
  say "Run '${APP_NAME} --help' to get started."
}

main "$@"