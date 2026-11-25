#!/usr/bin/env bash
# Build a fully static ak820-ctl AND a self-contained ak820-gui (libusb built in).
#
# Produces:
#   target/x86_64-unknown-linux-musl/release/ak820-ctl — CLI, no shared libs at all
#   target/release/ak820-gui                           — GUI (eframe/egui), host libc
#
# Neither binary needs libusb or libudev at link or runtime: `rusb`'s `vendored`
# feature compiles libusb from source into .o files, and we archive them into the
# libusb-1.0.a that hidapi's build script asks the linker for. The CLI has no
# shared-library dependencies at all (no glibc, no libX*, no GL).
#
# WHY THE GUI IS NOT FULLY STATIC (and cannot be):
# eframe loads its display-server and GL libraries at *runtime* with dlopen
# (libwayland-client.so, libGL, libX11, libxkbcommon — you can see their names in
# the binary). A fully static binary has no dynamic loader, and musl's static
# libc binds dlopen to `stub_dlopen`, which always fails. So a static-musl GUI
# dies on every desktop, Wayland or X11, with:
#   Error: WinitEventLoop(Os(... error: WaylandError(Connection(NoWaylandLib))))
# `file` will still say "statically linked" and `ldd` will lie. Only the GUI's
# dynamic-loader requirement is unavoidable; keep it on the host target.
#
# Requires (Fedora):  sudo dnf install musl-gcc kernel-headers
# The musl target is added with: rustup target add x86_64-unknown-linux-musl
set -euo pipefail

MUSL_TARGET=x86_64-unknown-linux-musl

# 1. Kernel headers for the C build (libusb). musl-gcc has its own sysroot and
#    won't see /usr/include, so expose just the kernel headers via symlinks.
KHDR="$(mktemp -d)"
ln -sf /usr/include/linux "$KHDR/linux"
ln -sf /usr/include/asm "$KHDR/asm"
ln -sf /usr/include/asm-generic "$KHDR/asm-generic"

export CFLAGS="-I$KHDR"
export PKG_CONFIG_PATH=/usr/lib64/pkgconfig
export PKG_CONFIG_ALLOW_CROSS=1

# Vendored libusb objects -> libusb-1.0.a, for the subdir of `target/` given as
# an argument (e.g. the musl tree or the host `release` tree).
stash_libusb() {
  local sub="$1" USBLIB objs
  USBLIB="$(mktemp -d)"
  objs="$(find "target/$sub" -name '*.o' -path '*libusb1-sys*' 2>/dev/null)"
  [ -n "$objs" ] || { echo "error: no vendored libusb objects under target/$sub" >&2; exit 1; }
  ar rcs "$USBLIB/libusb-1.0.a" $objs
  export RUSTFLAGS="-C link-arg=-L$USBLIB"
}

# All builds are release builds.
BUILD=(--release)

# $1 = the subdir of target/ whose libusb objects to archive, rest = cargo args.
build() {
  local sub="$1"; shift
  # Pass 1: compile deps. The final link fails (no libusb-1.0.a yet) — expected.
  cargo build "${BUILD[@]}" "$@" 2>&1 | tail -3 || true
  stash_libusb "$sub"
  # Pass 2: relink with deps cached.
  cargo build "${BUILD[@]}" "$@"
}

# --- CLI: fully static musl, no eframe, so no display stack at all. ---
build "x86_64-unknown-linux-musl" --target "$MUSL_TARGET" --bin ak820-ctl

# --- GUI: host target (dynamic loader), libusb still baked in. ---
build "release" --bin ak820-gui --features gui

echo
for BIN in "target/$MUSL_TARGET/release/ak820-ctl" target/release/ak820-gui; do
  echo "Binary: $BIN"
  file "$BIN"
  ldd "$BIN" || true
  echo "libusb linked in:"
  nm "$BIN" | grep -E ' T (libusb_init|libusb_open|libusb_get_version)$' || echo "  (missing!)"
  echo
done
