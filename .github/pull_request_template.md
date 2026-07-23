<!-- CoSign / 协签:合并需集齐 Agent 证据矩阵 + CI 全绿 + CODEOWNERS sign-off。门禁全貌见 docs/cosign.md -->

## 背景 / What & Why

> 一句话说明这个 PR 交付了什么,以及为什么做。

Closes COW-XX  <!-- 每个关联 issue 单独一行 -->

---

## 验收标准对照 — COW-XX

> 源自 [COW-XX](https://linear.app/clawmint/issue/COW-XX) 的完成标准,逐条核对。

| # | 完成标准(原文) | 状态 | 证据 / 说明 |
|---|---|---|---|
| 1 | ... | ✅/🟡/⬜/🔬 | `path/to/file.rs:line` 或命令输出 |

**小结**: N 条中 ✅ a / 🟡 b / 🔬 c / ⬜ d。⬜/🔬 项是本 PR 合并后仍需跟进的。

---

## 变更说明 / How

> 改了哪些文件,为什么这么改(不要复述 diff)。

---

## 验证 / How it was verified

> 只写本会话**真实跑过**的验证。没跑过的写 🔬。

- [ ] `cargo test -p api-server --all-targets` 通过
- [ ] `flutter analyze` 0 error
- [ ] `flutter test` 通过

---

## MPC / 密钥 / 安全(改动触及下列路径时必填)

> 触及 `crates/mpc-core`、`crates/storage-crypto`、`routes/{tx,auth,wallets,mpc}.rs`、
> `services/{crypto,presign_manager}.rs`、`ai_executor/` 时勾选并给证据。

- [ ] **密钥分片**:MPC 密钥材料未以明文出现在日志/响应/数据库明文字段
- [ ] **签名 + 策略门禁**:交易路径经过 policy-engine 评估;fail-closed 路径未被绕过
- [ ] **AI 执行器安全**:AI 工具调用不可被 prompt injection 绕过安全检查

---

## 部署注意 / Risk & Rollout

> 有无 DB migration?env 变更?不可逆操作?App/Server 必须同版本发布?

- 无 / 有(说明): ...
