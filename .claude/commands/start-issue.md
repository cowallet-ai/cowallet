# 开始实现 Issue(Linear → 代码)

喂一个 COW-xx 编号或链接,拉取需求 → 起分支 → 落地实现 → 对照「完成标准」验证 → 交棒。

## 输入

issue ID / URL / 分支名: $ARGUMENTS

解析规则:
- 完整 ID `COW-12` → 直接用
- URL `https://linear.app/clawmint/issue/COW-12/...` → 取出 `COW-12`
- 裸数字 `12` → 补成 `COW-12`
- 留空 → 从当前分支名推断;推断不出就**停下来问**,不要瞎猜

## 执行流程

1. **拉取需求**
   - `mcp__linear-server__get_issue`(完整 ID),取「目标/背景/范围/完成标准」
   - 记下 `gitBranchName`(Linear 给的标准分支名)
   - **若 issue 没有「完成标准」** → 停下来告知用户,问要不要先补全;无验收标准不开工

2. **认领**
   - `mcp__linear-server__save_issue`:assignee=me、state=In Progress

3. **起分支**
   - `git status` 确认工作区干净;不干净 → 停下来问
   - `git fetch origin && git switch -c <branch> origin/main`
   - 分支名用 `gitBranchName`(过长可截到 `pengxjwawa/cow-XX-<短描述>`)

4. **先读后写**(关键)
   - 把范围内每个文件路径都 Read 一遍,确认字段名/函数名真实存在
   - 触及 MPC 签名/分片/策略时:额外读相邻实现,理解 fail-closed 边界
   - 改动大或有取舍 → 先列计划让用户确认;改动小且路径明确 → 直接做

5. **实现**
   - 匹配周边代码风格/库,不随意引新依赖
   - Rust:注释英文,公开 API 写 doc comment
   - Flutter:遵循已有 widget/service 分层,不混入业务逻辑到 view 层
   - 改 mpc-core/chain-evm/policy-engine → 必配测试

6. **对照完成标准逐条验证**

   | 状态 | 含义 |
   |---|---|
   | ✅ 已满足 | 本会话真实跑过,给出命令+输出或文件:行 |
   | 🟡 部分 | 覆盖了哪部分,缺哪部分 |
   | ⬜ 未覆盖 | 本 PR 不解决 |
   | 🔬 未验证 | 代码写了但未在本会话验证 |

   - 能跑的现在跑:`cargo test -p <crate>` / `flutter test` / `flutter analyze`
   - 需联网/docker/真机 → 标 🔬,写明用哪条命令在哪个环境验
   - 有 ❌ 不达标 → 回第 5 步修,不带病交棒

7. **交棒**
   - 提示走 `/create-commit` 整理提交、`/create-pr` 生成 PR 描述
   - 在 Linear issue 留进展评论:已实现哪些、✅/🔬 状态、分支名

## 约束

- **先读后写**:动手前必须 Read 范围内真实文件
- **不夸大验证**:✅ 只给本会话真跑过的
- **不擅自提交/推送**
- **不擅自扩大范围**:偏离方向→停下来确认
- **认领即可见**:开工前设 In Progress + assignee
- **语言**:代码/提交英文,交流和 Linear 留言中文
