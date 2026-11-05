#!/bin/sh
# Установка ardor-mouse из архива релиза: sudo ./install.sh  (удаление: sudo ./install.sh --uninstall)
set -e
cd "$(dirname "$0")"
PREFIX=${PREFIX:-/usr/local}
if [ "$1" = "--uninstall" ]; then
    rm -f "$PREFIX/bin/ardor-mouse" "$PREFIX/share/applications/ardor-mouse.desktop" \
        /etc/udev/rules.d/70-ardor-mouse.rules "$PREFIX/share/icons/hicolor/scalable/apps/ardor-mouse.svg"
    for s in 16 32 48 64 128 256 512; do rm -f "$PREFIX/share/icons/hicolor/${s}x${s}/apps/ardor-mouse.png"; done
    udevadm control --reload-rules 2>/dev/null || true
    echo "ardor-mouse removed"
    exit 0
fi
install -Dm755 ardor-mouse "$PREFIX/bin/ardor-mouse"
install -Dm644 ardor-mouse.desktop "$PREFIX/share/applications/ardor-mouse.desktop"
install -Dm644 70-ardor-mouse.rules /etc/udev/rules.d/70-ardor-mouse.rules
install -Dm644 icons/ardor-mouse.svg "$PREFIX/share/icons/hicolor/scalable/apps/ardor-mouse.svg"
for s in 16 32 48 64 128 256 512; do
    install -Dm644 "icons/ardor-mouse-$s.png" "$PREFIX/share/icons/hicolor/${s}x${s}/apps/ardor-mouse.png"
done
udevadm control --reload-rules 2>/dev/null || true
udevadm trigger --subsystem-match=hidraw 2>/dev/null || true
echo "ardor-mouse installed to $PREFIX/bin. If the receiver was plugged in, replug it once."
