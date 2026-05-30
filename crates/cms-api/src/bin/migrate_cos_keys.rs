//! 一次性迁移：把“桶名双写”的旧对象 key（`<bucket>/variants/...`）服务端复制成
//! 干净 key（`variants/...`），再按各 photo 的 privacy 重刷 ACL。
//!
//! 背景：历史上 S3 client 的 endpoint 里带了桶名子域，又开了 `force_path_style(true)`，导致
//! 桶名被写进了对象 key（详见 services/media_service.rs 与 bootstrap.rs 的注释）。换自定义域名
//! 后，干净路径 `<自定义域名>/variants/...` 指向的 key 不存在 → 403。本工具把对象搬到
//! 干净 key 上，配合 endpoint 改成地域级（`https://cos.<region>.myqcloud.com`）即可彻底修好。
//!
//! 复制而非移动 → 旧对象留作回滚备份，确认站点正常后再去 COS 控制台批量删 `<bucket>/` 前缀。
//!
//! 用法（环境变量需指向**地域级** endpoint + 自定义域名；以下全是占位符）：
//! ```bash
//! APP_S3__ENDPOINT=https://cos.<region>.myqcloud.com \
//! APP_S3__PUBLIC_BASE_URL=https://<你的自定义域名> \
//! APP_S3__REGION=<region> \
//! APP_S3__BUCKET=<bucket> \
//! APP_S3__ACCESS_KEY=... APP_S3__SECRET_KEY=... \
//! cargo run -p cms-api --bin migrate_cos_keys
//! ```
//! 干跑（只打印映射、不动数据）：`... cargo run -p cms-api --bin migrate_cos_keys -- --dry-run`

use cms_api::{bootstrap, config, observability, services::photo_service};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::init();
    let dry_run = std::env::args().any(|arg| arg == "--dry-run");

    let cfg = config::load()?;
    let bucket = cfg.s3.bucket.clone();
    // 旧 key 多带的那层前缀就是“桶名/”。
    let prefix = format!("{bucket}/");
    let state = bootstrap::AppState::init(cfg).await?;

    println!(
        "migrate start: bucket=`{bucket}` strip-prefix=`{prefix}` dry_run={dry_run}"
    );

    // 1) 翻页列出所有“以桶名为前缀”的旧对象，逐个服务端复制到干净 key。
    let mut copied = 0usize;
    let mut token: Option<String> = None;
    loop {
        let mut req = state
            .s3
            .list_objects_v2()
            .bucket(&bucket)
            .prefix(&prefix);
        if let Some(t) = &token {
            req = req.continuation_token(t);
        }
        let resp = req.send().await?;

        for obj in resp.contents() {
            let Some(old_key) = obj.key() else { continue };
            let Some(clean_key) = old_key.strip_prefix(&prefix) else {
                continue;
            };
            if clean_key.is_empty() {
                continue; // 前缀本身（“目录占位”），跳过
            }

            if dry_run {
                println!("[dry-run] {old_key}  ->  {clean_key}");
                copied += 1;
                continue;
            }

            // CopySource = `bucket/旧key`（旧 key 安全字符集，无需额外编码）。
            let copy_source = format!("{bucket}/{old_key}");
            state
                .s3
                .copy_object()
                .bucket(&bucket)
                .copy_source(&copy_source)
                .key(clean_key)
                .send()
                .await
                .map_err(|error| {
                    anyhow::anyhow!("copy `{old_key}` -> `{clean_key}` failed: {error}")
                })?;
            copied += 1;
            if copied % 50 == 0 {
                println!("  copied {copied} objects...");
            }
        }

        match resp.next_continuation_token() {
            Some(t) => token = Some(t.to_owned()),
            None => break,
        }
    }
    println!("copied {copied} objects (prefix `{prefix}` stripped).");

    // 2) COPY 出来的新对象默认 private，按各 photo 的 privacy 重刷 ACL。
    if dry_run {
        println!("[dry-run] skip ACL resync.");
    } else {
        let n = photo_service::resync_all_acl(&state).await?;
        println!("re-synced ACL for {n} photos.");
    }

    println!(
        "done. 旧对象（带 `{prefix}` 前缀）仍保留作备份；确认站点图片正常后，可在 COS 控制台批量删除它们。"
    );
    Ok(())
}
