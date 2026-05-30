# gathered-light（拾光集）

> 一个轻量的图片展示与管理 Headless CMS，给个人摄影师 / 记录者用。
> 前台是慢节奏的瀑布流影像手记，后台是 Ghost 风的内容管理。

---

## 简介

**gathered-light** 是一套 Rust 后端 + React 前端的图片 CMS，类似 WordPress 之于博客 —— 但专注于「一张张图片 + 元数据」这种内容形态：

- 公开端：极简瀑布流影像，支持分类筛选 / 加锁分享 / 私密
- 管理端：账户体系、媒资上传到 OSS、自动生成多尺寸缩略图、标签 / 分类管理、批量操作
- 多语言：中英双语字段，前台可切换

设计灵感参考 Are.na、Substack 编辑感和 Ghost CMS 后台的留白美学。

---

## 技术栈

| 层 | 选型 | 备注 |
|---|---|---|
| Web 框架 | [Axum](https://github.com/tokio-rs/axum) | Tokio 生态，极简 + 高性能 |
| ORM | [SeaORM](https://github.com/SeaQL/sea-orm) | 体验贴近 MyBatis-Plus 的异步 ORM |
| 数据库 | PostgreSQL 18 | 主数据 |
| 缓存 / 黑名单 | Redis 7 | JWT 主动失效 + 热门列表缓存 |
| 对象存储 | S3 兼容（MinIO / 腾讯云 COS / AWS S3） | 后端签发 Presigned URL，前端直传 |
| 图片处理 | [`image`](https://crates.io/crates/image) crate | 异步 worker 生成 thumb_400 / medium_900 / full_1800 / webp_900 |
| 日志 / 追踪 | `tracing` + JSON 结构化输出 | 对标 SLF4J + Skywalking |
| 鉴权 | JWT HS256 + Redis 黑名单 | Access 15min / Refresh 7d |
| 前端 | React 18 + Vite + TypeScript | SPA |

---

## 功能

### 公开端
- 瀑布流影像浏览（带分类筛选）
- Lightbox 单图查看（移动端工具条底栏布局）
- 加锁照片需口令解锁；私密照片仅 owner 可见
- 中英双语切换
- 响应式适配，移动端友好

### 管理端
- 邮箱 + 密码登录 + JWT 续签
- Dashboard 概览（总数 / 按隐私 / 按分类 / 媒资队列状态）
- 照片：分页 / 搜索 / 筛选 / 批量删除 / 批量改隐私 / 批量打标
- 上传：客户端 PUT 直传 OSS（不经服务器），自动生成 4 个尺寸变体
- 标签 / 分类管理（CRUD + 排序）
- 用户管理（Owner / Editor / Viewer 三级角色）
- 多用户协作，Owner 可重置他人密码

---

## 项目结构

```
.
├── crates/
│   ├── cms-api/           # Axum 应用：handlers / services / repositories / middleware / workers
│   ├── cms-domain/        # 领域模型（Privacy 等纯类型）
│   └── cms-entity/        # SeaORM Entity 定义
├── migrations/            # 数据库迁移（独立二进制）
├── frontend/              # React + Vite SPA
│   ├── src/
│   │   ├── components/    # PublicGallery / AdminApp / Icons
│   │   ├── api/           # 统一 fetch 客户端
│   │   ├── hooks/         # useLang / useDebounce
│   │   ├── i18n.ts        # 中英文 token
│   │   └── styles.css     # 单文件 CSS（带 admin 主题作用域）
│   └── dist/              # 构建产物
├── config/
│   └── default.toml       # 应用默认配置
├── docker-compose.yml     # dev 用：PG + Redis + MinIO
├── scripts/
│   ├── dev-up.sh          # 本地一键起依赖 + cms-api + DoD 验证
│   ├── server-bootstrap.sh # 服务器一次性环境初始化
│   └── server-build.sh    # 服务器编译 + 部署
├── design/                # 原型 (HTML + babel-jsx 静态页)
└── docs/
    └── ARCHITECTURE.md    # 详细架构 / 数据模型 / 部署
```

Rust 后端的分层映射到 Spring：

| Rust crate | Spring 概念 |
|---|---|
| `handlers/` | `@RestController` |
| `services/` | `@Service` |
| `repositories/` | `@Repository` + MyBatis Mapper |
| `middleware/` | `HandlerInterceptor` / `Filter` |
| `dto/` | `*Req` / `*Resp` DTO |
| `infra/` | RedisTemplate / S3Client wrapper |
| `workers/` | `@Async` 后台任务 |

---

## 本地开发

### 0. 依赖

需要：
- Rust 1.85+（推荐 [USTC rustup 镜像](https://mirrors.ustc.edu.cn/help/rust-static.html)）
- Node 20+
- Docker + Docker Compose

### 1. 起依赖容器

```bash
bash scripts/dev-up.sh --skip-dod
# 起 PG (5432) + Redis (6379) + MinIO (9000/9001)
```

也可以手动：
```bash
docker compose up -d postgres redis minio
```

### 2. 跑迁移

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/gathered_light \
  cargo run -p migrations -- up
```

### 3. 创建 owner 账号

```bash
cp .env.example .env
cargo run -p cms-api --bin seed_admin -- you@local.dev 'Dev12345!'
```

### 4. 起后端 + 前端

后端：
```bash
cargo run -p cms-api
# 默认监听 0.0.0.0:8080；或 APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api
```

前端：
```bash
cd frontend
npm install
npm run dev
# Vite dev server 在 5173，自动代理 /api/* 到后端
```

打开 [http://localhost:5173](http://localhost:5173)。Admin 入口在 `/admin`。

### 可选：填一些演示数据

```bash
cargo run -p cms-api --bin seed_demo
# 写入 26 张 demo 照片 + 默认 categories / tags
# 加锁照片的口令是 1234
```

---

## 部署到生产

详见 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) 的部署章节。简化版：

```bash
# 服务器一次性 init（首次）
bash scripts/server-bootstrap.sh

# 拉代码 + 编译 + 拷贝产物
cd /opt/gathered-light
git clone <repo-url> src
cd src
bash scripts/server-build.sh

# 写 .env、起容器、跑迁移、seed
cd /opt/gathered-light
cp .env.example .env && $EDITOR .env
docker compose --env-file .env up -d postgres redis
DATABASE_URL=... ./migrations up
DATABASE_URL=... ./seed_admin you@yourdomain.com 'StrongPass!'

# systemd 启动
sudo systemctl enable --now cms-api

# nginx + HTTPS（自带 nginx 范本，见 docs/ARCHITECTURE.md）
sudo certbot --nginx -d yourdomain.com --redirect
```

### 生产环境 .env 注意

- `APP_BIND_ADDR=127.0.0.1:8081` 只绑 loopback，由 nginx 反代
- `APP_JWT__SECRET=$(openssl rand -base64 48)` 必须用强随机
- 对象存储用腾讯云 COS / AWS S3，**endpoint 写区域域名，不要带桶名**
  - 例：`https://cos.<region>.myqcloud.com`，不是 `https://your-bucket.cos.<region>.myqcloud.com`
- COS 桶必须配置 CORS，允许你的前端域名 PUT / GET / POST

---

## 关键设计点

### 1. JWT + Redis 黑名单
登出 / 改密码时把 token 的 jti 写入 Redis（TTL = 剩余过期时间），中间件每次请求查一次黑名单。比单 JWT 强，比纯 session 灵活。

### 2. OSS 上传链路
```
Admin 选文件 → POST /admin/media/presign → 后端返回 1h 有效的 PUT URL
            → 浏览器 PUT 直传 COS（不经服务器）
            → POST /admin/media/complete → 后端 head_object 校验大小 + mime
            → media_assets 入库 status=pending
            → 异步 worker 拉原图 → 生成 4 个 variants → status=ready
            → 前台 list 接口返回 medium_900 的 presigned URL（不直接暴露原图）
```

### 3. Headless 多语言
所有用户可见字段（title / loc / caption / alt_text）都是 `{zh, en}` 结构，前端按语言切换。后端不做翻译，纯存储。

### 4. 缓存策略
- 公开 list：Redis 缓存 5 分钟，按 query 参数 hash 作 key
- 写操作（create / update / delete）触发 `cms:photo:list:*` 前缀失效

---

## 路线图

- [ ] 多 variants 暴露给前端，用 `<picture>` + `srcset` 让浏览器选 webp / 不同尺寸
- [ ] 加锁照片支持自定义口令（当前硬编码 `1234`）
- [ ] 全文搜索接入 Meilisearch
- [ ] 评论 / 反应（按需）
- [ ] RSS / Atom feed
- [ ] 国际化扩展到日 / 韩

---

## 许可证

Apache License 2.0，详见 [LICENSE](LICENSE)。
