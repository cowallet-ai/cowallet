#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Rebuild ALL Flutter Rust Bridge FFI artifacts after changing Rust code.
#
# WHY THIS EXISTS: the Dart bindings (mobile/lib/bridge/frb_generated/) and the
# native libraries (Android .so in jniLibs, iOS xcframework) are build products
# of crates/ffi-mobile. If you edit any Rust under crates/ (api.rs, mpc-core,
# etc.) and DON'T rebuild these, the app runs stale native code — which has
# caused "RustLib has not been initialized" and DKG/protocol failures where the
# bindings referenced symbols the old .so didn't have.
#
# RUN THIS after every change to crates/ffi-mobile or its dependencies.
#
# Usage:
#   ./scripts/rebuild_ffi.sh              # codegen + android + ios (release)
#   ./scripts/rebuild_ffi.sh --android    # codegen + android only
#   ./scripts/rebuild_ffi.sh --ios        # codegen + ios only
#   ./scripts/rebuild_ffi.sh --bindings   # codegen only
#
# Requires (per platform): flutter_rust_bridge_codegen, cargo, rustup targets,
# cargo-ndk + Android NDK (android), Xcode (ios).
# ─────────────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

DO_ANDROID=1
DO_IOS=1
case "${1:-}" in
  --android)  DO_IOS=0 ;;
  --ios)      DO_ANDROID=0 ;;
  --bindings) DO_ANDROID=0; DO_IOS=0 ;;
  "" )        ;;  # default: both
  * ) echo "unknown arg: $1"; exit 1 ;;
esac

log() { echo ""; echo "==> $*"; }

# 1. Sanity: Rust must compile before we generate bindings / cross-compile.
log "cargo check -p ffi-mobile (fail fast before codegen / cross-compile)"
( cd "$SCRIPT_DIR/.." && cargo check -p ffi-mobile )

# 2. Regenerate Dart bindings so they match the current Rust API surface.
log "Generating Dart bindings (flutter_rust_bridge_codegen)"
"$SCRIPT_DIR/generate_bindings.sh"

# 3. Android native libs → mobile/android/app/src/main/jniLibs/<abi>/libffi_mobile.so
if [[ "$DO_ANDROID" == "1" ]]; then
  log "Building Android native libraries (.so for all ABIs)"
  "$SCRIPT_DIR/build_android.sh"
fi

# 4. iOS xcframework → mobile/ios/Frameworks/ffi_mobile.xcframework
if [[ "$DO_IOS" == "1" ]]; then
  if command -v xcodebuild >/dev/null 2>&1; then
    log "Building iOS xcframework"
    "$SCRIPT_DIR/build_ios.sh" release --no-codesign
  else
    log "SKIP iOS: xcodebuild not found (not on macOS with Xcode). Run scripts/build_ios.sh on a Mac."
  fi
fi

log "FFI rebuild complete. Now: cd mobile && flutter run"
