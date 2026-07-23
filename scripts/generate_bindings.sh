#!/usr/bin/env bash
# Generate Dart bindings from Rust FFI using flutter_rust_bridge

set -e

echo "🔧 Generating flutter_rust_bridge bindings..."

# Change to the mobile directory (script lives in scripts/, so go up one level).
cd "$(dirname "$0")/../mobile"

# Run code generation. Paths come from mobile/flutter_rust_bridge.yaml
# (rust_input: crate::api, rust_root: ../crates/ffi-mobile/,
#  dart_output: lib/bridge/frb_generated/), so invoking with no path flags uses
# that config — the same one that produced the committed bindings.
flutter_rust_bridge_codegen generate

echo "✅ Bindings generated successfully!"
