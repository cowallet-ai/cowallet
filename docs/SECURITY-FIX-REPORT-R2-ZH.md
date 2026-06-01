# cowallet —— 安全修复报告（第二轮）

> **文档编号**：`COWALLET-SECFIX-R2-ZH`
> **修复日期**：2026-06-01
> **范围**：本轮 16 项安全发现（编号 F-005 ~ F-025，与第一轮 `FENZ-2026-0142` **不同套**，详见下方说明）
> **修复状态**：**16/16 已实施并验证**

---

## ⚠️ 关于编号（重要）

本报告的 `F-0xx` 编号 **独立于** 既有的 `FENZ-2026-0142-SOLUTION-ZH`（第一轮审计）。两套编号语义不同，**切勿混用**：

| 编号 | 本轮（R2）语义 | FENZ 第一轮语义 |
|---|---|---|
| F-015 | 金额精度（f64 丢精度） | 明文 HTTP |
| F-016 | EVM 地址校验（EIP-55） | Covalent 密钥硬编码 |
| F-017 | AI 注入过滤器加固 | 同左（一致） |

引用本报告发现时，请带上 `COWALLET-SECFIX-R2` 前缀以消歧。

---

## 整体验证结论

| 验证项 | 结果 |
|---|---|
| `cargo build --workspace` | ✅ 通过（仅既有 dead-code 警告） |
| `cargo test -p mpc-core`（含签名套件） | ✅ 18 passed, 0 failed |
| `cargo test -p api-server crypto::` | ✅ 23 passed, 0 failed（含 2 个新增 AAD 绑定测试） |
| `cargo test -p policy-engine` | ✅ 46 passed, 0 failed |
| `cargo test -p chain-evm`（EntryPoint 一致性） | ✅ passed |

> **修复纠错记录**：F-017 曾被任务台账误标为「策略引擎失败关闭」，据此误改 `policy-engine/rules.rs` 的默认分支为硬 Deny，破坏了 deny-list 模型（合规交易不匹配任何规则、依赖 `RequireBiometric` 默认放行）。已 **完整还原**，policy-engine 恢复 46/46。F-017 的真实修复是 `ai.rs` 注入过滤器（代码本已存在）。

---

## 第一级 —— 认证与会话

### F-009：JWT 黑名单查询失败时被当作「未拉黑」→ 失败关闭

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | 黑名单校验在 DB/Redis 出错或缺失时返回「未拉黑」，被撤销的令牌可在故障窗口内继续使用。 |

**修复**（`middleware/auth.rs:236`）：黑名单查询 **失败关闭**——DB 错误或缺库一律视为「拒绝」，而非放行。

### F-010：设备绑定未强制 + 令牌入 WS 查询串 → 强制绑定 + 一次性票据

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | 请求未携带 `X-Device-ID` 时绕过设备绑定；WebSocket 把原始 JWT 放查询串（易入日志/代理缓存）。 |

**修复**：
- `middleware/auth.rs:254`：缺失 `X-Device-ID` 的请求 **直接拒绝**（设备绑定强制）。
- `routes/mpc_ws.rs:52` + `routes/auth.rs:692`：WS 改用 **一次性票据**（`POST /api/v1/auth/ws-ticket` 换取），不再在查询串传 JWT。

### F-011：限速器失败开放 + IP 可伪造 + OTP 阈值差一 → 失败关闭 + 真实 socket + 阈值修正

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | Redis 故障时限速器放行；信任客户端 `X-Forwarded-For`；OTP 锁定用 `>` 导致多放一次猜测；刷新端点的设备绑定取自令牌自身，校验恒为真（空操作）。 |

**修复**：
- `middleware/rate_limit.rs:156,208`：Redis 连接/查询失败 **失败关闭**。
- `middleware/rate_limit.rs:420`：不再信任客户端 `X-Forwarded-For`，按真实 peer socket 计数。
- `routes/auth.rs:229,970`：锁定阈值由 `>` 改为 `>=`，第 5 次失败即锁。
- `middleware/auth.rs:158,185,201` + `routes/auth.rs:597`：刷新端点设备绑定改用 **独立来源** 的 `X-Device-ID` 头（`presented_device_id`），而非被刷新令牌自带的 `device_id`——旧逻辑用令牌的 `device_id` 与自身比对恒为真，绑定形同虚设，被盗 refresh 令牌可在任意设备刷新；现缺头即返回 `FORBIDDEN`。

### F-014：客户端提供的身份字段作授权根 → 改从 JWT 取身份 + 令牌类型声明

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | `wallet_address`/`user_id`/`auth_method` 取自请求体；access/refresh 令牌无类型区分，可互换滥用。 |

**修复**：
- `middleware/auth.rs:13,167`：令牌带 `token_type` 声明；仅真正的 refresh 令牌可在刷新端点交换（`auth.rs:230` 拒绝 bearer 重放）。
- `routes/ai.rs:855,864,1127`：身份与钱包地址 **一律从已验证 JWT（`claims.sub`）解析**，忽略请求体。
- 兼容说明（`auth.rs:33`）：F-014 之前签发的无 `token_type` 旧令牌按 access 处理。

## 第二级 —— 越权与归属（IDOR/BOLA）

### F-007：推送注册 IDOR → 归属取自 JWT

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | 推送 token 注册信任请求体里的 `user_id`，任意用户可把推送绑到他人设备。 |

**修复**（`routes/push.rs:81`）：忽略请求体 `user_id`，归属取自已验证 JWT。

### F-008：交易历史 IDOR → 调用方须拥有交易一方

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | 历史/详情端点未校验调用方是否拥有该地址或交易。 |

**修复**（`routes/tx_history.rs:67,203,273,368` + `routes/tx.rs:274`）：调用方必须拥有该地址（或交易的 from/to 一方），否则返回 `NOT_FOUND`（不泄露存在性）。

### F-019：userOp `sender` 无归属校验 → 绑定到认证用户

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | 提交 ERC-4337 userOp 时 `sender` 未校验归属，可代他人发起账户抽象交易。 |

**修复**（`routes/userop.rs:150` + `routes/tx.rs:778`）：userOp 的 `sender` 必须属于认证用户。

---

## 第三级 —— AI 执行器与提示注入

### F-013：分片加密未绑定用户身份 → AES-GCM AAD 身份绑定

| | |
|---|---|
| **严重程度** | 🔴 严重 |
| **根因** | 所有服务器分片在同一静态根密钥下加密，密文与所属用户无密码学绑定——数据库内分片行被挪到他人记录仍可解密。 |

**修复**：
- `services/crypto.rs:70,122,139`：新增 `derive_key_with_salt` / `encrypt_bound` / `decrypt_bound`——以「所属身份」既作 AES-GCM AAD（认证但不入密文）又作 HKDF salt（每用户密钥相异）。
- `services/mpc_participant/shard_store.rs:23,28,35,47`：六处读写全部接入身份 AAD（`aad_user` / `aad_wallet`）；`decrypt_stored` 按持久化的 `encryption_key_id`（`-aad` 后缀）做 **版本分派**——仅旧方案行走非绑定回退路径，新分片无法被降级绕过。
- **新增测试**：`crypto::tests::test_aad_binding_rejects_foreign_identity`、`..._distinct_keys_per_identity`——验证为用户 A 加密的分片不能以用户 B 身份解密。

### F-015：金额按 f64 换算丢精度 → 精确整数换算

| | |
|---|---|
| **严重程度** | 🟠 高危 |
| **根因** | 人类可读金额经 `amount * 10^decimals as u128` 用 f64 中转，悄悄改变用户已批准的金额。 |

**修复**（`services/ai_executor.rs:124` `parse_decimal_to_smallest`）：纯整数解析，拒绝非法/超精度输入，不再经 f64。

### F-016：地址仅查长度前缀 → EIP-55 校验 + 规范化

| | |
|---|---|
| **严重程度** | 🟠 高危 |
| **根因** | `to_address`/`contract_address` 仅检查 `0x` 前缀与长度，混合大小写时不校验 EIP-55 校验和。 |

**修复**（`services/ai_executor.rs:160` `validate_evm_address`）：校验 hex + 混合大小写时强制 EIP-55，返回规范化 checksum 形式；F-013 的「用户输入地址交叉校验」用 `to_lowercase()` 比较，规范化后仍正确匹配。

### 附加：ERC-20 授权竞态 + USDT 兼容（无审计编号，资金可用性修复）

| | |
|---|---|
| **严重程度** | 🟠 高危（资金流可用性） |
| **根因** | 兑换前的 `approve` 流程有二：① 对 USDT 类代币在 **非零** allowance 上直接改额会 `revert`（这类代币要求先置 0）；② approve 后固定 `Future.delayed(3s)` 即发 swap，链上未确认时 allowance 尚未生效，swap 因授权不足失败（竞态）。 |

**修复**（`mobile/lib/services/intent_executor.dart`）：
- 授权前若 `currentAllowance > 0`，先发 `approve(spender, 0)` 重置并等待确认，再发无限授权——兼容 USDT 类代币。
- 新增 `_waitForTxConfirmation`（轮询交易回执，最多约 60s）取代固定 3 秒延时；approve **确认后** 才发 swap，消除竞态。

### F-017：注入过滤器易绕过且扫错内容 → 归一化加固（已存在，本轮核对）

| | |
|---|---|
| **严重程度** | 🟡 中危 |

**说明**：真实修复位于 `routes/ai.rs:507,535,608`（`normalize_for_threat_scan` / `detect_threat` / `detect_threat_in_context`），代码本轮核查已存在。**不涉及策略引擎默认值**（见首部纠错记录）。

---

## 第四级 —— MPC 核心内存安全

### F-024：FFI 全局锁中毒后永久不可用 → 容忍中毒

| | |
|---|---|
| **严重程度** | 🟠 高危 |
| **根因** | `ffi-mobile/state.rs` 全局会话表用 `.lock().unwrap()`，任一持锁线程 panic 即毒化锁，后续所有 MPC 操作 panic，钱包整体瘫痪。 |

**修复**（`crates/ffi-mobile/src/state.rs`，13 处）：全部改为 `.lock().unwrap_or_else(|e| e.into_inner())`，中毒后仍恢复内部数据继续工作。

### F-025：签名会话敏感材料不清零 → Zeroize + Drop

| | |
|---|---|
| **严重程度** | 🟠 高危 |
| **根因** | `SignSession` 不像 `DkgSession`/`ReshareSession` 那样在析构时清零，临时签名 nonce `my_k`、Paillier 私钥、密钥分片用后仍驻留内存——`my_k` 泄露 + 签名即可反推长期分片。 |

**修复**（`crates/mpc-core/src/dkls23/sign.rs:54,74`）：实现 `Zeroize`/`Drop`，清零 `my_k`/`r_scalar`，并取出 `my_share`/`paillier_keypair` 触发其各自的清零析构。

---

## 第五级 —— 卫生级

### F-005：`.env.example` 含真实密钥 → 占位符

| | |
|---|---|
| **严重程度** | 🟠 高危 |
| **根因** | `.env.example` 直接写入真实 DeepSeek API Key 与可用的 dev `JWT_SECRET`。 |

**修复**（`.env.example:85,109`）：替换为占位符 `sk-your-deepseek-api-key-here` / `change-me-generate-with-openssl-rand-base64-32`。
> ⚠️ **需运维执行**：该 DeepSeek 密钥已存在于 git 历史，**必须在 DeepSeek 控制台轮换/吊销**——仅改文件无法撤回。生产 `JWT_SECRET` 须用 `openssl rand -base64 32` 生成的强密钥。

### F-021：移动端日志打印令牌内容 → 全部脱敏

| | |
|---|---|
| **严重程度** | 🟡 中危 |
| **根因** | 三处 `print` 输出 JWT 全文或前缀（前缀仍泄露 header/alg 并缩小爆破空间）。 |

**修复**：`mobile/lib/api/example_usage.dart:20`（全文→`<redacted>`）、`mobile/lib/network/dio_client.dart:47`（前缀→`Token attached`）、`mobile/lib/api/auth_api.dart:52`（前缀→`<received>`）。

### F-023：EntryPoint 版本不一致 → 统一到 v0.6

| | |
|---|---|
| **严重程度** | 🟡 中危 |
| **根因** | 签名/提交路径全程实现 ERC-4337 **v0.6**（`UserOperation` 扁平布局、`hash()` 注 v0.6、硬编码 v0.6 EntryPoint），但 `chains.rs` 声明了 **v0.7** EntryPoint（`erc4337_entrypoint` 字段，实际从未被读取）。若日后接入签名路径，会与 v0.6 打包/哈希不匹配，产出被 bundler 拒绝的签名。 |

**修复**（用户决策：统一到 v0.6）：`crates/chain-evm/src/chains.rs` 8 处地址、7 处 `.expect` 文案、1 处测试注释统一为 v0.6（`0x5FF1…2789`）。`test_entrypoint_addresses_consistent` 通过。

---

## 运维待办（代码层之外）

| 项 | 动作 | 原因 |
|---|---|---|
| **F-005** | 在 DeepSeek 控制台 **轮换/吊销** 泄露的 API Key | 密钥已入 git 历史，删文件无法撤回 |
| **F-005** | 生产 `JWT_SECRET` 用 `openssl rand -base64 32` 重新生成 | `.env.example` 旧值为弱 dev 值 |
| **F-013** | 评估对存量旧方案分片做离线重加密迁移 | 当前靠版本门兼容旧分片；迁移后可移除非绑定回退路径 |

## 待确认（本轮范围外）

- **F-012**（预言机不可用时 USD 限额坍缩为 $0 → 失败关闭）：`FENZ-2026-0142` 文档标 CONFIRMED-FIXED（`services/ai_executor.rs:~795`）。本轮未触碰，请确认已覆盖。

---

## 附：本轮 16 项一览

| 编号 | 主题 | 关键文件 | 状态 |
|---|---|---|---|
| F-005 | 移除泄露密钥 | `.env.example` | ✅ 代码层 |
| F-007 | 推送注册 IDOR | `routes/push.rs` | ✅ |
| F-008 | 交易历史 IDOR | `routes/tx_history.rs`,`tx.rs` | ✅ |
| F-009 | JWT 黑名单失败关闭 | `middleware/auth.rs` | ✅ |
| F-010 | 设备绑定 + WS 票据 | `middleware/auth.rs`,`routes/mpc_ws.rs` | ✅ |
| F-011 | 限速失败关闭 + 阈值 | `middleware/rate_limit.rs`,`routes/auth.rs` | ✅ |
| F-013 | 分片身份绑定（AAD） | `services/crypto.rs`,`shard_store.rs` | ✅ +测试 |
| F-014 | 身份取自 JWT + 令牌类型 | `middleware/auth.rs`,`routes/ai.rs` | ✅ |
| F-015 | 金额精度 | `services/ai_executor.rs` | ✅ |
| F-016 | EIP-55 地址校验 | `services/ai_executor.rs` | ✅ |
| F-017 | 注入过滤器（已存在） | `routes/ai.rs` | ✅ 核对 |
| F-019 | userOp 归属 | `routes/userop.rs`,`tx.rs` | ✅ |
| F-021 | 移动端令牌日志脱敏 | `mobile/lib/...` | ✅ |
| F-023 | EntryPoint 统一 v0.6 | `crates/chain-evm/src/chains.rs` | ✅ +测试 |
| F-024 | FFI 锁中毒容忍 | `crates/ffi-mobile/src/state.rs` | ✅ |
| F-025 | 签名会话清零 | `crates/mpc-core/src/dkls23/sign.rs` | ✅ +测试 |


