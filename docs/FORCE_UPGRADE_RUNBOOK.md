# 强制升级发版 Runbook（v1.0.1+）

> 适用于任何引入**不向后兼容**的客户端变更（如 v1.0.1 的 MPC 签名协议改动）、
> 需要把旧版本 App 强制挡在门外的发版。

## 背景

CoWallet 采用**客户端 + 服务端双层强制升级**：

- **客户端层（提示）**：App 启动时请求 `GET /api/v1/config/app-version`,
  若本机 build 号 < `min_build`,跳转不可返回的升级墙 `ForceUpgradeView`。
  失败 fail-open（网络/解析异常照常进入 App）。
- **服务端层（硬拦截）**：所有受保护路由（签名/交易/swap/钱包）校验请求头
  `X-App-Version`,低于 `MIN_APP_BUILD` 直接返回 **426 Upgrade Required**。
  绕过/缓存的旧客户端也签不了名。

**build 号** = `mobile/pubspec.yaml` 中 `version:` 的 `+` 后整数
（`1.0.1+17` → `17`）,整数比较,单调递增。

## 🚨 铁律:先上架,后拉门槛

`MIN_APP_BUILD=N` 会 426 拦掉所有 build < N 的客户端。**必须**等新 build 在
**双端商店都已上架可下载**后,才能把门槛拉到 N。顺序反了 = 全体用户被锁在
升级墙外、却无新版可下 = 生产事故。

因此门槛分两段:
1. 部署带 gate 代码,但 `MIN_APP_BUILD=0`（fail-open,不拦任何人）。
2. 双端上架通过后,只改 env 把门槛拉到新 build,重启即可(无需重新编译)。

## 环境变量

| 变量 | 含义 | 默认 |
|---|---|---|
| `MIN_APP_BUILD` | 低于此 build 强制升级(426) | `0`=不拦 |
| `LATEST_APP_BUILD` | 最新 build(预留软提示) | `0` |
| `IOS_STORE_URL` | iOS 升级跳转 | 空(需填真实数字 ID) |
| `ANDROID_STORE_URL` | Android 升级跳转 | Play Store 链接 |

服务器 `.env` 落地,由 `docker-compose.yml` / `docker-compose.prod.yml` 的
`api-server.environment` 透传进容器。

---

## 发版步骤

### Step 1 — 后端发布（门槛仍为 0）

1. 把本次发版分支合入 `main`（`deploy.sh` 是 `git reset --hard origin/main`）。
2. 确认服务器 `.env` 中 `MIN_APP_BUILD=0`（本次先不拦人）。
3. 服务器执行:
   ```bash
   cd /opt/cowallet          # 你们的部署目录
   ./deploy.sh               # 拉码 + rebuild + up + 健康检查 + 认证冒烟
   ```
4. 验证新接口已上线且 fail-open:
   ```bash
   curl -s localhost:3000/api/v1/config/app-version
   # 期望: {"min_build":0,"latest_build":0,"ios_store_url":...,"android_store_url":...}
   ```

### Step 2 — 移动端构建并提审（build 17）

1. 确认 `mobile/pubspec.yaml` 为目标版本(当前 `1.0.1+17`)。
2. **重编 FFI 原生产物(改过 `crates/` 时必做)**。
   Android `.so`(`jniLibs/`)与 iOS `ffi_mobile.xcframework` 是 `crates/ffi-mobile`
   的构建产物,**不由 `flutter build` 自动编译**(gradle 无 cargokit 钩子)。若本次
   动过任何 `crates/`(如 v1.0.1 的 `mpc-core` 签名协议改动),旧产物会让 App 打进
   过期的 native 代码 —— 强升级后仍是旧协议,与新后端签不了名。
   ```bash
   ./scripts/rebuild_ffi.sh         # codegen + Android .so(4 架构)+ iOS xcframework
   ```
   > 判断是否需要:`git diff --stat main...HEAD -- crates/`,有输出即必须重编。
   > 纯 Dart 改动(如本强升级功能)不需要;但本次 release 动了 `mpc-core`,必做。
3. 构建 App 包:
   ```bash
   cd mobile && flutter pub get
   ./build-android-prod.sh          # Android release(AAB/APK)
   ../scripts/build_ios.sh          # iOS release(或 Xcode Archive)
   ```
4. 提交 App Store 与 Play Store 审核。
5. 审核期如需评审绕过登录,用 compose 里现成的 `REVIEW_BYPASS_EMAIL`/
   `REVIEW_BYPASS_OTP`,**审核通过后清空并重新部署**。

### Step 3 — 双端上架通过 → 拉高门槛（gate 生效）

> 前置:build 17 在 **iOS 与 Android 商店都已可下载**。

改服务器 `.env`(不用重新编译):
```bash
MIN_APP_BUILD=17
LATEST_APP_BUILD=17
IOS_STORE_URL=https://apps.apple.com/app/id<真实数字ID>
ANDROID_STORE_URL=https://play.google.com/store/apps/details?id=com.cowallet.app

docker compose up -d api-server    # env 变更重启即生效
```

### Step 4 — 验证 gate

```bash
# 1) 公开接口返回新门槛(老 App 仍能拉到,用于渲染升级页)
curl -s localhost:3000/api/v1/config/app-version
#    期望 min_build:17 + 两个商店链接

# 2) 模拟旧客户端(build 16)访问受保护路由 → 426
curl -s -o /dev/null -w "%{http_code}\n" \
  -H "X-App-Version: 16" -H "Authorization: Bearer <token>" \
  localhost:3000/api/v1/wallets          # 期望 426

# 3) 模拟新客户端(build 17) → 非 426
curl -s -o /dev/null -w "%{http_code}\n" \
  -H "X-App-Version: 17" -H "Authorization: Bearer <token>" \
  localhost:3000/api/v1/wallets          # 期望 200/401,不应是 426
```

真机验证:装 build 16 启动 → 弹不可返回的升级墙,点"立即更新"跳对应商店;
装 build 17 → 正常进入钱包。

---

## 回滚

**优先**(秒级、无需编译):把门槛降回 0 解除全体拦截,再排查。
```bash
# 服务器 .env
MIN_APP_BUILD=0
docker compose up -d api-server
```

代码回滚(仅当 gate 逻辑本身有 bug):
```bash
git reset --hard <上一个正常 commit> && ./deploy.sh
```

---

## 发版检查清单

- [ ] 发版分支已合入 `main`
- [ ] `.env` 中 `MIN_APP_BUILD=0`,后端已部署,`/config/app-version` 返回 200
- [ ] `pubspec.yaml` build 号已递增
- [ ] 改过 `crates/` 时已 `./scripts/rebuild_ffi.sh` 重编 `.so`/xcframework
- [ ] Android / iOS release 已构建并提审
- [ ] **双端商店均已上架可下载**（拉门槛的前置条件）
- [ ] `IOS_STORE_URL` 已填真实数字 App Store ID
- [ ] `.env` 门槛拉高至新 build 并重启
- [ ] Step 4 三条 curl + 双真机验证通过
- [ ] 评审绕过变量(如启用过)已清空并重部署

---

## 常见坑

- **门槛不生效、谁都不拦**:多半是容器没读到 env。确认对应 compose 的
  `api-server.environment` 有这 4 个变量(base 与 prod 都要),且 `.env` 已加载。
- **iOS 升级按钮 toast 报错跳不走**:`IOS_STORE_URL` 为空。填真实数字 ID。
- **新 App 也被拦**:门槛设得比新 build 还高。门槛须 ≤ 已上架的最新 build。
- **改了 env 不生效**:需 `docker compose up -d api-server` 重启容器,`restart`
  不重新读 `.env`。
- **升级后仍签不了名 / "RustLib has not been initialized"**:打包前漏跑
  `rebuild_ffi.sh`,App 里是过期的 `.so`/xcframework。改过 `crates/` 必须先重编
  再打包(见 Step 2)。
