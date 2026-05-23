# AGENTS.md — Codex / 任意 AI 代理首读

> 这一页是给 **Codex / Claude Code / 任何自动化代理** 的**强制阅读入口**。
> 不读完就开始改代码 = 不合格交付。

## 第 1 条：唯一真理来源 = `docs/ARCHITECTURE.md`

本项目的设计、数据模型、API 形状、缓存约定、PR 任务规格、验收命令都在 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)。

- `CLAUDE.md`：用户身份 / 角色 / 高层愿景。
- `AGENTS.md`（本文件）：Codex 流程入口 + 速读硬约束。
- `docs/ARCHITECTURE.md`：**真正的工程契约**。

**三者冲突时一律以 `docs/ARCHITECTURE.md` 为准。**

## 第 2 条：进门动线（先后顺序，不要跳）

1. 读完 [`docs/ARCHITECTURE.md` §0.5 接力须知](docs/ARCHITECTURE.md#05-接力须知--handoff-manifestcodex--任何-ai-接力代理-必读)
2. 读完 [§12 硬约束 H-1..H-10](docs/ARCHITECTURE.md#12-硬约束hard-constraints--不达标即不合格)
3. 读完 [§13 当前仓库状态](docs/ARCHITECTURE.md#13-当前仓库状态诚实评估) — **了解哪里是 mock**
4. 按 [§11 Quickstart](docs/ARCHITECTURE.md#11-quickstart本地开发环境) 起 docker-compose（PG 18 + Redis 7 + MinIO）
5. 进入 [§14 M1 任务规格](docs/ARCHITECTURE.md#14-m1-任务规格--auth--db--redis-接入pr-by-pr) 找当前 PR
6. 写代码前先扫一眼 [§20 禁止模式](docs/ARCHITECTURE.md#20-禁止模式forbidden-patterns) + [§21 冻结决策](docs/ARCHITECTURE.md#21-冻结决策frozen--不接受讨论) + [§22 接力速查](docs/ARCHITECTURE.md#22-接力速查给-codex--后续-ai-代理的常见踩坑指南)

## 第 3 条：硬约束速读（详见 §12）

> 这一节是**完整规则的索引**，不能替代 §12 阅读。出现冲突以 §12 为准。

| # | 一句话 |
| --- | --- |
| **H-1** | 主存储 = PostgreSQL 18 + SeaORM。**严禁** in-memory `Vec`/`HashMap` 替代 |
| **H-2** | JWT 主动失效**必须**走 Redis（`auth:blacklist:{jti}` + `auth:user-rev:{uid}`）。**严禁** `Arc<HashSet>` |
| **H-3** | 密码 = argon2id。**严禁**明文 / **严禁**密码入仓 / **严禁** `config/*.toml` 写密码 |
| **H-4** | S3 直传 = 真签名（`aws-sdk-s3` v1+）。**严禁** mock URL / `MOCK-*` 字样 |
| **H-5** | 图片处理 = `tokio::spawn` worker 异步。**严禁** handler 内同步处理 |
| **H-6** | API 路径 / 字段名遵从 §3 + §9，**改名要先同步前端** |
| **H-7** | 分层纪律：handler 不写 SQL；service 不直接调 redis-client；鉴权走 Tower middleware **一次挂上**，不在每个 handler 里 `require_access` |
| **H-8** | 错误统一 `AppResult<T>`；**严禁** `.unwrap()` / `.expect()` / `panic!` 在生产路径 |
| **H-9** | 每个 service 函数挂 `#[tracing::instrument]`；缓存命中/未命中用 `tracing::debug!` 记 |
| **H-10** | 前端契约（§9 字段对照表）只能加字段、不能改名 / 不能删字段 |

## 第 4 条：每个 PR 的 5 行交付检查清单

复制以下模板进 PR 描述（不可省略）：

```markdown
## DoD checklist
- [ ] 命中的硬约束编号：H-?, H-?
- [ ] 引入的禁止模式（应为空）：F-? — 若非空，请论证为何不可避免 + 后续替换 PR
- [ ] DoD 验收命令贴出 + **实际输出**（见 docs/ARCHITECTURE.md §14 各 PR 末尾）
- [ ] 前端契约（§9）是否破坏：是 / 否（若是，需同步改 frontend/src/api/client.ts）
- [ ] 是否新增 tracing span / 业务字段：是 / 否
```

## 第 5 条：当前仓库状态（一句话）

**M0 已交付**（workspace + Axum + tracing + healthz）。Codex 此前提交了一份 in-memory 原型 + React 前端，**多数后端能力违反 §12 硬约束**（详见 §13.2）。**当前任务**：按 §14 PR-1 → PR-5 把所有 mock 替换为合规实现，前端 API 形状（§9）保持不变。

## 第 6 条：你不知道答案时

- ❌ **不要**自己揣测（默认走"最低阻力路径"会违反硬约束）
- ✅ 在 PR 描述里 `## Open questions` 列出，等用户回答

—— 入门到此为止，请前往 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)。
