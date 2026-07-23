# 验证部署(合并后 ECS 上线核验)

合并到 main 后,核验 ECS 服务已拉到新镜像并健康运行,不绿不推 Done。

## 输入

环境提示 / 留空: $ARGUMENTS

- 留空 → 产出核验命令脚本(本会话通常无法直接 SSH ECS)
- 给了主机/别名 → 生成在该主机执行的命令
- `--local` → 核验本地 `docker compose` 栈

## 背景

合并 ≠ 上线。ECS 部署由 `deploy-ecs.yml` 触发,分为 build → push ECR → deploy(更新 task definition)。每次发布需核验同一套探针,值得固化。

## 执行流程

> 本会话通常无法直接访问 ECS,默认**产出可粘贴的核验脚本**,不声称自己跑过。
> 仅当确认有本地 docker 且 compose 可运行时,直接执行。

1. **确认部署版本**
   ```bash
   # 查看 ECS task 正在跑的 image tag
   aws ecs describe-tasks --cluster <cluster> --tasks $(aws ecs list-tasks --cluster <cluster> --query 'taskArns[0]' --output text) \
     --query 'tasks[0].containers[0].image'
   # 本地:
   git log --oneline -1  # 记下 commit
   ```

2. **探针断言**
   ```bash
   BASE=https://<your-domain>
   # 健康
   curl -sf "$BASE/health"
   # 就绪(DB/NATS/Redis 全通)
   curl -sf "$BASE/ready"
   # 存活
   curl -sf "$BASE/live"
   # 指标(确认服务正常采集)
   curl -sf "$BASE/metrics" | head -20
   ```

3. **MPC 服务可用断言(可选,需 JWT)**
   ```bash
   # 检查预签名池状态(服务启动后应在后台填充)
   curl -sf -H "Authorization: Bearer $JWT" "$BASE/api/v1/mpc/presign/status"
   ```

4. **判定 + 回写**
   - 全绿 → 在对应 Linear issue `mcp__linear-server__save_comment` 记一条:
     部署 commit + 核验结果 + 时间戳
   - 若 issue 已完成所有验收标准 → 调 `mcp__linear-server__save_issue` 推 Done
   - **任一红 → 不推 Done**,报告哪步失败,留在 In Review

## 核验清单

| 项 | 端点 | 通过标准 |
|---|---|---|
| 部署版本正确 | ECS 镜像 tag | 与预期 commit SHA 匹配 |
| 健康 | `/health` | `{"status":"ok",...}` |
| 就绪 | `/ready` | HTTP 200 |
| 存活 | `/live` | HTTP 200 |
| 指标采集 | `/metrics` | 非空,含 cowallet_* 指标 |

## 约束

- **不假装跑过**:无 ECS 访问的环境只产出脚本
- **不绿不推 Done**:只有真拿到全绿输出才推 Done
- **记录部署 commit**:每次核验记下实际跑的 commit
