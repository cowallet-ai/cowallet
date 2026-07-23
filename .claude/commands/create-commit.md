# 创建 Commit(标准化提交)

把当前改动整理成一个或多个原子提交,消息遵循 conventional commits + co-author。

## 输入

可选范围/意图: $ARGUMENTS

- 留空 → 提交全部已暂存改动;若无暂存,审视所有变更并智能分组
- 给了提示(「只提交 auth 相关」「拆成两个」)→ 按提示约束范围

## 执行流程

1. **看清现状**
   - `git status --short`、`git diff --stat`、`git diff --cached`
   - 无改动 → 停下来告知,不空提交

2. **判断原子性**
   - 一个提交 = 一个逻辑主题
   - 同一主题的代码+测试+文档在**同一提交**
   - 多主题 → 分批暂存分批提交;不确定时列方案让用户确认

3. **暂存**
   - `git add <具体路径>`,**绝不** `git add .` / `git add -A`
   - 暂存前扫一遍:`.env`、密钥、`*.log`、`target/`、调试产物 → 命中即停警示

4. **写消息**(见格式)

5. **提交**
   - `git commit -F -` + heredoc 传完整消息(避免转义问题)
   - 保留 hooks,**不加** `--no-verify`(除非用户明确要跳)
   - 提交后 `git log --oneline -3` 回显

## 消息格式

```
<type>(<scope可选>): <主题,祈使句,≤70字符,英文>

<正文:为什么改 + 改了什么要点。每行≤72字符。>

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
```

### type

| type | 用于 |
|---|---|
| feat | 新功能 |
| fix | 修 bug |
| chore | 脚手架/配置/CI/依赖 |
| docs | 仅文档 |
| refactor | 不改外部行为的重构 |
| test | 仅加/改测试 |
| ops | 部署/运维 |
| perf | 性能优化 |

### 主题行规则
- 祈使句、现在时:`add presign mutex` 不是 `added`
- 英文、小写开头、结尾不加句号
- ≤70 字符

## 约束

- **绝不** `git add .` / `git add -A`
- **绝不**裹入密钥/`.env`/`target/`/大体积产物
- **绝不**擅自 `git push`
- **绝不**擅自 `--amend` 已推送提交
- **绝不**加 `--no-verify`,除非用户明确要
