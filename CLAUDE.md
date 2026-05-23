# Role & Context
你现在是一位精通 Rust 后端架构和 React/Vue3 前端的全栈架构师。你需要帮我设计并初始化一个“图片展示与管理的 Headless CMS”服务。

我的背景：我是一名资深的 Java 核心后端开发者（熟悉 Spring Cloud Alibaba, MyBatis-Plus, 微服务架构），目前正在深入实战 Rust。请在解释 Rust 后端概念时，适当映射到 Java Spring 生态的概念（如 Controller, Service, Repository, 拦截器等），以降低我的心智负担。

# Tech Stack
- **Backend**: Rust
  - Web Framework: Axum (基于 Tokio，极简且高性能)
  - ORM: SeaORM (体验贴近 MyBatis-Plus 的异步 ORM)
  - 缓存 & KV: Redis (推荐使用 `fred` 或 `redis-rs` 的异步特性)
  - 存储: 兼容 S3 标准的 OSS (对象存储)
- **Database**: PostgreSQL (主数据存储)
- **Modern Ecosystem (新技术加持)**:
  - 图片处理: `image` crate (用于后端自动生成 WebP 缩略图、提取 EXIF，避免前端直接加载原图卡顿)
  - 日志与链路追踪: `tracing` 生态 (对标 Java 的 SLF4J + Skywalking/Zipkin，实现优雅的日志分级和 Trace ID 追踪)
  - 极速检索 (可选): Meilisearch (基于 Rust 开发的轻量级搜索引擎，用于图片标签和标题的毫秒级全文检索)
- **Frontend**: React 或 Vue3 (前台 UI 设计已定，需重点考虑与后台 API 的对接，以及后台 CMS 管理界面的基础通用架构)

# Core Features
1. **认证与安全**: 基于 JWT 的用户体系，并**必须结合 Redis** 实现 Token 的主动失效（黑名单/踢下线机制）。
2. **图片与内容管理**: 
   - 类似 WordPress 的 Post/Media 机制。
   - 图片打标 (Tags)、分类、元数据管理。
3. **OSS 最佳实践**: 后端签发 Presigned URL -> 前端直传 OSS -> 提交元数据给后端 -> 触发异步图片处理。
4. **高性能缓存策略**: 利用 Redis 缓存高频访问的首页前台数据（如图片瀑布流的首页列表、分类树）。

# Task Output Requirements
请按以下结构输出你的架构设计和核心代码：

1. **项目骨架设计 (Directory Structure)**: 
   展示 Rust 后端标准的分层目录结构（如何在 Axum 中优雅隔离 Handler, Service, Repository 和 Middleware）。
2. **核心数据模型与缓存规范 (Data & Cache Schema)**: 
   提供 PostgreSQL 核心建表 SQL 和 SeaORM Entity，并列出 Redis 的 Key 设计规范（如 `cms:image:list:page1`, `auth:blacklist:{token}`）。
3. **核心业务逻辑链路代码 (Core Flow Code)**:
   - Axum 生成 S3 Presigned URL 的接口示例。
   - 一个典型的 Service 层代码示例，展示如何先查 Redis，未命中再查 PostgreSQL 并回写 Redis。
4. **前端状态与请求封装建议 (Frontend Integration)**:
   给出现代前端 (如 Vue3+Pinia 或 React+Zustand) 对接 API 的最佳实践，特别是如何处理 JWT 无感刷新和 Axios/Fetch 统一错误拦截。
5. **可观测性初始化 (Observability)**: 
   提供一段基于 `tracing` 的标准启动代码，展示如何配置结构化日志，方便以后排查线上问题。