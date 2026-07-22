# CoSign / 协签 — 代码合并门禁

> **代码合并,如同一笔 MPC 交易——需要多方协签,少一片都不放行。**
>
> cowallet 的钱包内核是 2-of-3 门限签名(TSS):一笔交易要多个分片协同签名才放行。
> 一个改动想合进 `main` 同理,必须集齐三方「协签」:**Agent 层**(验收标准证据矩阵)、
> **CI 层**(audit/gitleaks/test 全绿)、**人类 CODEOWNERS**(资金/MPC 路径 sign-off)。
> 任一方不签 = 合并不成立。与 fail-closed 结算语义同构。

守护 `main` 的检查项。两层结构:**Agent 命令化审核** + **确定性 CI 门禁**。
全部开源免费,无付费 SaaS。

## 每个 PR 上跑什么

| 层 | 工具 | Workflow | 门禁 |
|---|---|---|---|
| 构建 + 测试(Rust) | cargo test | `ci.yml` | required |
| 依赖漏洞(Rust) | cargo audit (RustSec) | `rust-audit.yml` | required |
| 静态分析(Rust) | cargo clippy | `rust-audit.yml` | advisory |
| 移动端分析 + 测试 | flutter analyze/test | `flutter-ci.yml` | required(mobile 变更时) |
| 密钥扫描 | gitleaks | `security.yml` | required(PR 新提交) |
| 本地 pre-commit | `.githooks/` | — | advisory |

`.github/CODEOWNERS` 额外把 MPC/资金/安全路径(`crates/mpc-core`、`crates/storage-crypto`、
`routes/{tx,auth,wallets,mpc}.rs`、`services/{crypto,presign_manager}.rs`、`ai_executor/`)的 review
路由给 maintainer。

## 为什么这样选(不照搬参考项目)

- **cargo audit 而非 CodeQL**:CodeQL 原生不支持 Rust。RustSec 咨询库是 Rust 生态的等价物。
- **clippy 先 advisory**:仓库有历史 lint debt,先 `continue-on-error` 不阻塞 PR;
  待 debt 清完移除该标志、升为 required(改 `rust-audit.yml` 的 clippy job)。
- **gitleaks PR 只扫新提交**:git 历史中有已知误报和一个已轮换的历史密钥(见下),
  PR 模式用 `--log-opts origin/<base>..HEAD` 只看新增,不因历史债务变红。

## gitleaks

阻止密钥进入历史。直接跑 gitleaks CLI(官方 action 对 org 仓库需付费)。
配置见 [`.gitleaks.toml`](../.gitleaks.toml):扩展默认规则集,allowlist 了 `.env.example`
占位符、CocoaPods lock、Firebase 公开 client id。

本地运行:

```bash
brew install gitleaks
gitleaks detect --config .gitleaks.toml --redact
```

> ⚠️ **历史泄露**:`.env.prod.template` 曾硬编码真实 DeepSeek 生产密钥(现已替换为占位符)。
> 该密钥已进入 git 历史,**必须在 DeepSeek 控制台轮换**;彻底清除需 git-filter-repo/BFG 改写历史。

## 本地 pre-commit hook

零依赖,通过 git 共享。每次 clone 后启用一次:

```bash
git config core.hooksPath .githooks
```

每次提交:(1) 扫 staged diff 密钥(有 gitleaks 用之,否则 regex 回退);
(2) 对改到的 Rust crate 跑 `cargo check`,改到 `mobile/` 跑 `flutter analyze`。
紧急跳过:`git commit --no-verify`。它是 advisory —— CI 才是真门禁。

## Agent 命令化审核(`.claude/commands/`)

审核工作流的主干,沉淀项目知识:

```
/create-issue  → 建规范 issue(定义可执行验收标准)
/start-issue   → Linear → 分支 → 实现 → 逐条对照验收标准验证
/create-commit → 原子提交(conventional + co-author)
/create-pr     → 对照验收标准生成 ✅/🟡/⬜/🔬 证据矩阵(核心,诚实优先)
/fix-pr        → CI 红 / review 意见 / 落后 main → 本地复现修复重验
/verify-deploy → 合并后 ECS 上线核验,不绿不推 Done
```

核心原则:**诚实优先**——验收矩阵里 ✅ 只给本会话真跑过的,其余标 🔬/⬜。

## 启用 branch protection(需 maintainer 手动开)

`Settings → Branches → Add rule (main)`:
1. Require status checks:勾选 `Test (api-server)`、`cargo audit (RustSec)`、`gitleaks (secret scan)`
2. Require review from Code Owners(启用 CODEOWNERS 路由)
3. clippy/flutter debt 清完后再把对应 check 设为 required

## 未采用(及原因)

- **CodeRabbit / PR-Agent(外部 AI 审)** — 与本地 Claude `/create-pr` 审核重叠,单人仓库徒增噪音。团队变大再议。
- **CI 里调 LLM 审 diff** — 同上;审核逻辑放在 agent 命令里,可复现、可离线。
