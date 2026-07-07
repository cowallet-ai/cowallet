#!/usr/bin/env bash
# Run the cowallet Flutter app with build-time config from dart_define.json
# (e.g. TURNSTILE_SITE_KEY). Any extra args are passed through to flutter run,
# so you can still do: ./run.sh -d <device>, ./run.sh --release, etc.
set -e

cd "$(dirname "$0")"

exec flutter run --dart-define-from-file=dart_define.json "$@"
