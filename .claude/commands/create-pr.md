# 创建 PR(对照 Linear 验收标准)

为当前分支生成 PR 描述,**逐条对照 COW-xx 完成标准**,标注证据与状态,组装 PR 模板,给出 `gh pr create` 命令。

## 输入

issue 编号或留空: $ARGUMENTS

- 给了编号(如 `COW-12`,可多个空格分隔)→ 用这些
- 留空 → 从分支名推断(形如 `pengxjwawa/cow-12-...`);推断不出就停下来问,不要瞎猜

## 执行流程

1. **确定关联 issue**
   - 解析 $ARGUMENTS 或分支名得到编号
   - 对每个编号调用 `mcp__linear-server__get_issue`,取「完成标准」全部条目
   - 多 issue → 合并去重

2. **采集改动事实**(只读)
   - `git log --oneline main..HEAD`
   - `git diff --stat main..HEAD`
   - 需要时 `git diff main..HEAD -- <path>` 确认某条标准是否满足
   - **不运行**构建/测试(除非本会话已跑过)

3. **逐条对照 → 证据矩阵**

   | 状态 | 含义 | 要求 |
   |---|---|---|
   | ✅ 已满足 | 改动覆盖了这条 | 必须有证据:`path:line`、commit hash、或本会话真实跑过的输出 |
   | 🟡 部分 | 部分覆盖 | 说明覆盖了哪部分、缺哪部分 |
   | ⬜ 未覆盖 | 本 PR 不解决 | 明说,不混进"已完成" |
   | 🔬 未验证 | 代码写了但未在本会话验证 | 写明需要哪条命令/哪个环境 |

   **诚实优先**:宁可标 🔬/⬜ 也不要把没验证的写成 ✅。

4. **组装 PR 描述**
   - 读 `.github/pull_request_template.md` 作为骨架
   - 填「背景/What&Why」:一句话目标 + `Closes COW-XX`(每个关联 issue 一行)
   - 插入验收标准证据矩阵
   - 「验证」只写本会话**真实执行过**的;没跑的归到 🔬
   - 「MPC/密钥/安全」:改动触及敏感路径时逐项给证据
   - 「Risk&Rollout」:如实填(migration?env 变更?App/Server 同版本?)

5. **产出**(默认先展示,不直接建)
   - 打印完整 PR 描述
   - 给出命令:
     ```bash
     git push -u origin <当前分支>
     gh pr create --base main --title "<type>: <交付物>" --body-file /tmp/pr-<分支>.md
     ```
   - 若 $ARGUMENTS 含 `--create` → 直接调 `gh pr create`
   - 标题格式:`<type>: <一句话交付物>`,≤70 字符

## 证据矩阵格式

```markdown
## 验收标准对照 — COW-XX

> 源自 [COW-XX](https://linear.app/clawmint/issue/COW-XX) 完成标准。

| # | 完成标准(原文) | 状态 | 证据 / 说明 |
|---|---|---|---|
| 1 | ... | ✅ | `backend/api-server/src/routes/auth.rs:142` |
| 2 | ... | 🔬 | 需真机验证;命令:`cargo test --features integration-tests` |
| 3 | ... | ⬜ | 不含,见 COW-YY |

**小结**: 3 条中 ✅1 / 🔬1 / ⬜1。
```

## 约束

- **只读采集**:不修改源码、不提交、不强推
- **证据可点击**:引用文件用 `path:line`;issue 用完整 URL
- **不夸大**: `How it was verified` 里 ✅ 只能是本会话真发生过的
- **关联闭环**:`Closes COW-XX`(解决)或 `Refs COW-XX`(相关)
- **找不到 issue**:推断不出时停下来让用户给编号
- **语言**:PR 描述中文正文,type/scope 英文
