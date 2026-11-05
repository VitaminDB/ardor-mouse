#!/bin/sh
# Архив релиза для GitHub: packaging/release/build.sh → dist/ardor-mouse-<версия>-x86_64-linux.tar.gz (+ .sha256)
set -e
cd "$(dirname "$0")/../.."
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME="ardor-mouse-$VER-x86_64-linux"
cargo build --release --locked
rm -rf "dist/$NAME" && mkdir -p "dist/$NAME/icons"
install -m755 target/release/ardor_mouse "dist/$NAME/ardor-mouse"
strip "dist/$NAME/ardor-mouse"
cp packaging/ardor-mouse.desktop packaging/70-ardor-mouse.rules packaging/release/install.sh README.md LICENSE "dist/$NAME/"
cp assets/icon/ardor-mouse.svg assets/icon/ardor-mouse-*.png "dist/$NAME/icons/"
tar -C dist -czf "dist/$NAME.tar.gz" "$NAME"
(cd dist && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
echo "dist/$NAME.tar.gz"
