#!/usr/bin/env bash
# Build fully static, self-contained ak820-ctl AND ak820-gui binaries (musl).
#
# Produces, under target/x86_64-unknown-linux-musl/release/:
#   ak820-ctl  — CLI
#   ak820-gui  — GUI (eframe/egui)
#
# Both have NO shared-library dependencies (no libusb, no libudev, no glibc,
# no libX11/libwayland/libGL at link time). They run on any x86-64 Linux box
# without installing libusb/hidapi. The GUI loads X11/Wayland/GL at runtime
# via dlopen, so it only needs those present on a normal desktop.
#
# Requires (Fedora):  sudo dnf install musl-gcc kernel-headers
# The musl target is added with: rustup target add x86_64-unknown-linux-musl
set -euo pipefail

TARGET=x86_64-unknown-linux-musl

# 1. Kernel headers for the C build (libusb). musl-gcc has its own sysroot and
#    won't see /usr/include, so expose just the kernel headers via symlinks.
KHDR="$(mktemp -d)"
ln -sf /usr/include/linux "$KHDR/linux"
ln -sf /usr/include/asm "$KHDR/asm"
ln -sf /usr/include/asm-generic "$KHDR/asm-generic"

# 2. Where we'll stash the archived libusb static lib.
USBLIB="$(mktemp -d)"

export CFLAGS="-I$KHDR"
export PKG_CONFIG_PATH=/usr/lib64/pkgconfig
export PKG_CONFIG_ALLOW_CROSS=1
export RUSTFLAGS="-C link-arg=-L$USBLIB"

# Build flags: release, musl, GUI feature on, so BOTH bins are produced.
BUILD=(--release --target "$TARGET" --features gui)

# Pass 1: compile all deps. libusb1-sys (from `rusb`) vendors libusb and
# compiles it into .o files. The final link fails here (no libusb-1.0.a yet),
# which is fine — we only need the objects.
cargo build "${BUILD[@]}" 2>&1 | tail -3 || true

# Archive the vendored libusb objects into the name hidapi's build script
# asks the linker for (-lusb-1.0).
OBJS="$(find "target/$TARGET" -name '*.o' -path '*libusb1-sys*' 2>/dev/null)"
[ -n "$OBJS" ] || { echo "error: no vendored libusb objects found" >&2; exit 1; }
ar rcs "$USBLIB/libusb-1.0.a" $OBJS

# Pass 2: relink. Deps are cached, so this only links.
cargo build "${BUILD[@]}"

echo
for name in ak820-ctl ak820-gui; do
  BIN="target/$TARGET/release/$name"
  echo "Static binary: $BIN"
  file "$BIN"
  ldd "$BIN" || true
  echo
done
