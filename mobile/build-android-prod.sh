#!/usr/bin/env bash
#
# 正式环境 Android 构建脚本 (cowallet)
# ---------------------------------------------------------------------------
# 产出可上架的签名包：
#   默认           → App Bundle (.aab)，用于上传 Google Play
#   --apk          → 分架构 APK，用于侧载 / 直接分发
#
# 前置条件（缺失会直接报错，不会静默退回 debug 签名）：
#   - mobile/android/key.properties        （签名凭证，gitignored）
#   - mobile/android/app/upload-keystore.jks（或 key.properties 里指向的 keystore）
#   - mobile/android/app/google-services.json 属于正式 Firebase 项目
#
# build-time 配置（含 TURNSTILE_SITE_KEY）统一从 dart_define.json 读取，
# 与 run.sh 保持一致（flutter --dart-define-from-file）。
#
# 用法：
#   ./build-android-prod.sh                        # 打 AAB
#   ./build-android-prod.sh --apk                  # 打分架构 APK
#   ./build-android-prod.sh --define-file custom.json
#
set -euo pipefail

# --- 配置 -------------------------------------------------------------------
# 正式 Firebase 项目 id（用于校验 google-services.json，防止误用测试配置打正式包）
readonly EXPECTED_FIREBASE_PROJECT="cowallet-prod-43f1c"

# 脚本所在目录即 Flutter 项目根 (mobile/)
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# --- 参数解析 ---------------------------------------------------------------
BUILD_APK=false
DEFINE_FILE="dart_define.json"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --apk)          BUILD_APK=true; shift ;;
    --define-file)  DEFINE_FILE="${2:-}"; shift 2 ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//' | head -22
      exit 0 ;;
    *) echo "未知参数: $1" >&2; exit 2 ;;
  esac
done

# --- 颜色输出 ---------------------------------------------------------------
red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
blue()  { printf '\033[34m%s\033[0m\n' "$*"; }

fail() { red "✗ $*"; exit 1; }

# --- 前置校验 ---------------------------------------------------------------
blue "==> 校验构建前置条件"

command -v flutter >/dev/null 2>&1 || fail "flutter 未在 PATH 中"

# 1. 签名凭证必须存在（否则 build.gradle.kts 会退回 debug 签名，打出无法上架的包）
[[ -f android/key.properties ]] \
  || fail "缺少 android/key.properties —— 正式包必须用 upload keystore 签名"

# 2. keystore 文件必须存在（storeFile 相对 android/app/ 解析）
store_file="$(grep -E '^storeFile=' android/key.properties | cut -d= -f2- | tr -d '[:space:]')"
[[ -n "$store_file" ]] || fail "key.properties 未配置 storeFile"
if [[ "$store_file" = /* ]]; then
  keystore_path="$store_file"
else
  keystore_path="android/app/$store_file"
fi
[[ -f "$keystore_path" ]] || fail "keystore 不存在: $keystore_path"

# 3. google-services.json 必须是正式 Firebase 项目
gs_file="android/app/google-services.json"
[[ -f "$gs_file" ]] || fail "缺少 $gs_file"
actual_project="$(grep -o '"project_id": *"[^"]*"' "$gs_file" | head -1 | sed -E 's/.*"project_id": *"([^"]*)".*/\1/')"
[[ "$actual_project" == "$EXPECTED_FIREBASE_PROJECT" ]] \
  || fail "google-services.json 属于 '$actual_project'，期望正式项目 '$EXPECTED_FIREBASE_PROJECT'"

green "✓ 签名凭证: android/key.properties"
green "✓ keystore: $keystore_path"
green "✓ Firebase 项目: $actual_project"

# --- dart-define 文件校验 ---------------------------------------------------
[[ -f "$DEFINE_FILE" ]] \
  || fail "缺少 dart-define 文件: $DEFINE_FILE（含 TURNSTILE_SITE_KEY 等 build-time 配置）"

if grep -q '"TURNSTILE_SITE_KEY"' "$DEFINE_FILE"; then
  green "✓ dart-define 文件: $DEFINE_FILE (含 TURNSTILE_SITE_KEY)"
else
  red "⚠ $DEFINE_FILE 未含 TURNSTILE_SITE_KEY —— app 将跳过人机验证步骤。"
fi

# --- 构建 -------------------------------------------------------------------
blue "==> flutter clean"
flutter clean

blue "==> flutter pub get"
flutter pub get

app_version="$(grep -E '^version:' pubspec.yaml | awk '{print $2}')"
blue "==> 构建正式包 (version=$app_version)"

if $BUILD_APK; then
  flutter build apk --release --split-per-abi --dart-define-from-file="$DEFINE_FILE"
  green "✓ 构建完成 — APK:"
  ls -lh build/app/outputs/flutter-apk/*.apk 2>/dev/null || true
else
  flutter build appbundle --release --dart-define-from-file="$DEFINE_FILE"
  green "✓ 构建完成 — AAB (上传 Google Play):"
  ls -lh build/app/outputs/bundle/release/*.aab 2>/dev/null || true
fi

green "全部完成。"
