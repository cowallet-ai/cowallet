# 修复 PR(CI 失败 / 评审意见 / 同步 main)

处理已开 PR 的弹回:拉取 CI 失败日志与 review 意见 → 本地复现 → 修复 → 重验 → 同步 main。

## 输入

PR 编号 / 分支名 / 留空: $ARGUMENTS

- 给了 PR 号(如 `4`)→ 用它
- 给了分支名 → 找对应 PR
- 留空 → 用当前分支对应的 PR;找不到就停下来问

## 执行流程

1. **定位 PR + 拉取状态**
   - `gh pr view <pr> --json number,headRefName,baseRefName,mergeable,mergeStateStatus`
   - `gh pr checks <pr>` —— 哪些 check 红了
   - `gh pr view <pr> --comments` —— review 意见
   - 确认本地在该 PR 分支:`git switch <headRefName>`
   - **若无 gh/无网** → 停下来列出需用户在联网终端跑的命令,让用户把结果贴回来

2. **归类要处理的项**
   - **CI 失败**:按 check 名对应本地命令:
     - `Test (api-server)` → `cargo test -p api-server --all-targets`
     - `cargo audit` → `cargo audit`
     - `cargo clippy` → `cargo clippy --workspace --all-targets -- -D warnings`
     - `flutter analyze` → `cd mobile && flutter analyze`
     - `flutter test` → `cd mobile && flutter test`
     - `gitleaks` → `gitleaks protect --staged --config .gitleaks.toml --redact`
   - **Review 意见**:逐条列出,标「同意改/需澄清/不认同」—— 不认同的**回到用户确认**
   - **落后 main/冲突** → 第 4 步同步

3. **本地复现再修**(关键)
   - 先跑出同一个红,再动手修
   - 改完**重跑该命令确认变绿**
   - 触及 MPC/签名/策略路径的修复,重跑相关测试,不只 check

4. **同步 main(落后或冲突)**
   - `git fetch origin`
   - rebase 优先:`git rebase origin/main`
   - 冲突 → 逐文件解,`git add <file> && git rebase --continue`
   - 冲突涉及 MPC/签名/policy 逻辑 → **停下来人工确认**,不赌
   - rebase 后**必须重跑验证**
   - 推送:`git push --force-with-lease`(不用 `-f`)

5. **对照验收标准重验**
   - 改动可能影响之前 ✅ 的项,把关联 issue 完成标准重过一遍
   - 标 ✅/🔬,与 `/create-pr` 同口径

6. **交棒**
   - 修复提交用 `/create-commit`(`fix:` 前缀;正文写 "address review: <要点>")
   - 推送后在 PR 上 `gh pr comment <pr>` 回应每条 review 意见

## 约束

- **先复现再修**:不照 CI 日志盲改
- **修完必重验**:每个修复重跑对应命令确认变绿
- **force 用 lease**:只用 `--force-with-lease`,绝不 `git push -f`
- **不吞评审意见**:不认同的回到用户确认
- **不夸大**:✅ 只给本会话真跑过的
- **MPC/签名冲突必停**:合并冲突涉及密钥/签名逻辑,停下来人工确认
