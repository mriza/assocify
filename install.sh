#!/bin/bash
set -e

echo "🚀 Installing Assocify..."

# 1. Define user-local paths
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

mkdir -p "$BIN_DIR"
mkdir -p "$APP_DIR"
mkdir -p "$ICON_DIR"

# 2. Download binary from the "latest" release
TEMP_DIR=$(mktemp -d)
echo "📥 Downloading latest release binary..."
curl -sSL "https://github.com/mriza/assocify/releases/download/latest/assocify-linux-x86_64.tar.gz" -o "$TEMP_DIR/assocify.tar.gz"

echo "📦 Extracting..."
tar -xzf "$TEMP_DIR/assocify.tar.gz" -C "$TEMP_DIR"

echo "⚙️  Installing binary to $BIN_DIR..."
cp "$TEMP_DIR/assocify" "$BIN_DIR/assocify"
chmod +x "$BIN_DIR/assocify"

# 3. Install the icon from the archive
echo "🖼️  Installing icon..."
cp "$TEMP_DIR/assocify.svg" "$ICON_DIR/assocify.svg"

# 4. Create and install the .desktop file
echo "🖥️  Creating application shortcut..."
cat <<EOF > "$APP_DIR/assocify.desktop"
[Desktop Entry]
Name=Assocify
Comment=Modern Linux File Association Manager
Exec=$BIN_DIR/assocify
Icon=assocify
Terminal=false
Type=Application
Categories=Utility;Settings;DesktopSettings;
EOF

# Update the desktop database so the start menu sees it immediately
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications"
fi

# Cleanup
rm -rf "$TEMP_DIR"

echo "✅ Installation complete!"
echo "Assocify should now appear in your Start Menu / Application Launcher."
echo "Note: Ensure that $BIN_DIR is in your PATH if you want to run it from the terminal."
