#!/bin/bash
set -e

# Sort out OS
OS=$(uname -s)
if [[ "$OS" == "Linux" ]]; then
  OS_TYPE="linux"
elif [[ "$OS" == "Darwin" ]]; then
  OS_TYPE="macos"
elif [[ "$OS" == *"MINGW"* || "$OS" == *"CYGWIN"* ]]; then
  OS_TYPE="windows"
else
  echo "Unsupported OS: ${OS}"
  exit 1
fi
echo "OS: ${OS_TYPE}"

# Sort out architecture
ARCH=$(uname -m)
echo "Architecture: ${ARCH}"

if [[ "$ARCH" == "x86_64" ]]; then
  ARCH="x64"
elif [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
  ARCH="arm64"
else
  echo "Unsupported architecture: ${ARCH}"
  exit 1
fi

# Install base tailwindcss
BASE_ASSET="tailwindcss-${OS_TYPE}-${ARCH}"

echo "Fetching latest tailwind-cli release version..."
BASE_RELEASE_VERSION=$(curl -s https://api.github.com/repos/tailwindlabs/tailwindcss/releases/latest | jq -r '.tag_name')
echo "Latest tailwind-cli version: ${BASE_RELEASE_VERSION}"

echo "Installing tailwind-cli: ${BASE_RELEASE_VERSION} - ${BASE_ASSET}"
BASE_URL="https://github.com/tailwindlabs/tailwindcss/releases/download/${BASE_RELEASE_VERSION}/${BASE_ASSET}"
echo "Downloading tailwindcss from ${BASE_URL}"
curl -sLO "${BASE_URL}"
mv "${BASE_ASSET}" "/usr/local/cargo/bin/tailwindcss"
chmod +x "/usr/local/cargo/bin/tailwindcss"
/usr/local/cargo/bin/tailwindcss --help

# Install tailwindcss-extra
EXTRA_ASSET="tailwindcss-extra-${OS_TYPE}-${ARCH}"

echo "Fetching latest tailwind-cli-extra release version..."
EXTRA_RELEASE_VERSION=$(curl -s https://api.github.com/repos/dobicinaitis/tailwind-cli-extra/releases/latest | jq -r '.tag_name')
echo "Latest tailwind-cli-extra version: ${EXTRA_RELEASE_VERSION}"

echo "Installing tailwind-cli-extra: ${EXTRA_RELEASE_VERSION} - ${EXTRA_ASSET}"
EXTRA_URL="https://github.com/dobicinaitis/tailwind-cli-extra/releases/download/${EXTRA_RELEASE_VERSION}/${EXTRA_ASSET}"
echo "Downloading tailwindcss-extra from ${EXTRA_URL}"
curl -sLO "${EXTRA_URL}"
mv "${EXTRA_ASSET}" "/usr/local/cargo/bin/tailwindcss-extra"
chmod +x "/usr/local/cargo/bin/tailwindcss-extra"
/usr/local/cargo/bin/tailwindcss-extra --help
