//! Post（文章/页面）数据访问层。
//!
//! 多租户：每个操作都在事务内 `SET LOCAL app.tenant_id`（触发 Postgres RLS 硬隔离），
//! 并在查询条件里再带一遍 `tenant_id`（应用层防御 + 兼容 dev superuser 绕过 RLS 的情况）。
//! 详见 docs/USER_CENTER_AND_IAM_DESIGN.md §2.6。

use cms_entity::{post_revisions, post_terms, post_translations, posts};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
    prelude::DateTimeWithTimeZone,
};
use serde_json::Value;

pub struct NewTranslation {
    pub locale: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub body_json: Option<Value>,
    pub body_html: Option<String>,
    pub word_count: i32,
}

pub struct NewPost {
    pub site_id: i64,
    pub post_type: String,
    pub slug: String,
    pub status: String,
    pub visibility: String,
    pub author_id: Option<i64>,
    pub menu_order: i64,
    pub published_at: Option<DateTimeWithTimeZone>,
    pub translations: Vec<NewTranslation>,
    pub term_taxonomy_ids: Vec<i64>,
}

pub struct PostDetail {
    pub post: posts::Model,
    pub translations: Vec<post_translations::Model>,
    pub term_taxonomy_ids: Vec<i64>,
}

fn now() -> DateTimeWithTimeZone {
    chrono::Utc::now().fixed_offset()
}

/// 在事务内绑定 RLS 租户变量。tenant_id 来自服务端上下文（非用户输入），无注入风险。
async fn bind_tenant(txn: &DatabaseTransaction, tenant_id: i64) -> Result<(), DbErr> {
    txn.execute_unprepared(&format!("SET LOCAL app.tenant_id = '{tenant_id}'"))
        .await
        .map(|_| ())
}

pub async fn create(db: &DatabaseConnection, tenant_id: i64, input: NewPost) -> Result<i64, DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let post = posts::ActiveModel {
        tenant_id: Set(tenant_id),
        site_id: Set(input.site_id),
        post_type: Set(input.post_type.clone()),
        slug: Set(input.slug.clone()),
        status: Set(input.status.clone()),
        visibility: Set(input.visibility.clone()),
        author_id: Set(input.author_id),
        menu_order: Set(input.menu_order),
        published_at: Set(input.published_at),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    write_translations(&txn, tenant_id, post.id, &input.translations).await?;
    write_terms(&txn, tenant_id, post.id, &input.term_taxonomy_ids).await?;
    txn.commit().await?;
    Ok(post.id)
}

pub async fn update(
    db: &DatabaseConnection,
    tenant_id: i64,
    id: i64,
    input: NewPost,
) -> Result<(), DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let exists = posts::Entity::find()
        .filter(posts::Column::Id.eq(id))
        .filter(posts::Column::TenantId.eq(tenant_id))
        .one(&txn)
        .await?;
    if exists.is_none() {
        return Err(DbErr::RecordNotFound(format!("post {id}")));
    }
    posts::ActiveModel {
        id: Set(id),
        slug: Set(input.slug.clone()),
        post_type: Set(input.post_type.clone()),
        status: Set(input.status.clone()),
        visibility: Set(input.visibility.clone()),
        author_id: Set(input.author_id),
        menu_order: Set(input.menu_order),
        published_at: Set(input.published_at),
        updated_at: Set(now()),
        ..Default::default()
    }
    .update(&txn)
    .await?;
    post_translations::Entity::delete_many()
        .filter(post_translations::Column::PostId.eq(id))
        .exec(&txn)
        .await?;
    post_terms::Entity::delete_many()
        .filter(post_terms::Column::PostId.eq(id))
        .exec(&txn)
        .await?;
    write_translations(&txn, tenant_id, id, &input.translations).await?;
    write_terms(&txn, tenant_id, id, &input.term_taxonomy_ids).await?;
    txn.commit().await
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    tenant_id: i64,
    id: i64,
) -> Result<Option<PostDetail>, DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let post = posts::Entity::find()
        .filter(posts::Column::Id.eq(id))
        .filter(posts::Column::TenantId.eq(tenant_id))
        .one(&txn)
        .await?;
    let detail = match post {
        Some(post) => Some(load_detail(&txn, post).await?),
        None => None,
    };
    txn.commit().await?;
    Ok(detail)
}

pub async fn find_by_slug(
    db: &DatabaseConnection,
    tenant_id: i64,
    site_id: i64,
    post_type: &str,
    slug: &str,
) -> Result<Option<PostDetail>, DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let post = posts::Entity::find()
        .filter(posts::Column::TenantId.eq(tenant_id))
        .filter(posts::Column::SiteId.eq(site_id))
        .filter(posts::Column::PostType.eq(post_type))
        .filter(posts::Column::Slug.eq(slug))
        .one(&txn)
        .await?;
    let detail = match post {
        Some(post) => Some(load_detail(&txn, post).await?),
        None => None,
    };
    txn.commit().await?;
    Ok(detail)
}

/// 管理端列表（含草稿），按 post_type + 可选 status + slug 关键字过滤。
pub async fn list_admin(
    db: &DatabaseConnection,
    tenant_id: i64,
    site_id: i64,
    post_type: &str,
    status: Option<&str>,
    q: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<(Vec<PostDetail>, i64), DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let mut select = posts::Entity::find()
        .filter(posts::Column::TenantId.eq(tenant_id))
        .filter(posts::Column::SiteId.eq(site_id))
        .filter(posts::Column::PostType.eq(post_type));
    if let Some(status) = status.map(str::trim).filter(|s| !s.is_empty()) {
        select = select.filter(posts::Column::Status.eq(status));
    }
    if let Some(keyword) = q.map(str::trim).filter(|k| !k.is_empty()) {
        select = select.filter(posts::Column::Slug.contains(keyword));
    }

    let total = select.clone().count(&txn).await? as i64;
    let offset = page.saturating_sub(1) as u64 * page_size as u64;
    let rows = select
        .order_by_desc(posts::Column::PublishedAt)
        .order_by_desc(posts::Column::CreatedAt)
        .offset(offset)
        .limit(page_size as u64)
        .all(&txn)
        .await?;

    let mut out = Vec::with_capacity(rows.len());
    for post in rows {
        out.push(load_detail(&txn, post).await?);
    }
    txn.commit().await?;
    Ok((out, total))
}

/// 前台列表：仅 status=published 且 visibility=public。
pub async fn list_published(
    db: &DatabaseConnection,
    tenant_id: i64,
    site_id: i64,
    post_type: &str,
    page: u32,
    page_size: u32,
) -> Result<(Vec<PostDetail>, i64), DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let select = posts::Entity::find()
        .filter(posts::Column::TenantId.eq(tenant_id))
        .filter(posts::Column::SiteId.eq(site_id))
        .filter(posts::Column::PostType.eq(post_type))
        .filter(posts::Column::Status.eq("published"))
        .filter(posts::Column::Visibility.eq("public"));

    let total = select.clone().count(&txn).await? as i64;
    let offset = page.saturating_sub(1) as u64 * page_size as u64;
    let rows = select
        .order_by_desc(posts::Column::PublishedAt)
        .order_by_desc(posts::Column::CreatedAt)
        .offset(offset)
        .limit(page_size as u64)
        .all(&txn)
        .await?;

    let mut out = Vec::with_capacity(rows.len());
    for post in rows {
        out.push(load_detail(&txn, post).await?);
    }
    txn.commit().await?;
    Ok((out, total))
}

pub async fn delete(db: &DatabaseConnection, tenant_id: i64, id: i64) -> Result<(), DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let result = posts::Entity::delete_many()
        .filter(posts::Column::Id.eq(id))
        .filter(posts::Column::TenantId.eq(tenant_id))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    if result.rows_affected == 0 {
        return Err(DbErr::RecordNotFound(format!("post {id}")));
    }
    Ok(())
}

pub async fn slug_exists(
    db: &DatabaseConnection,
    tenant_id: i64,
    site_id: i64,
    post_type: &str,
    slug: &str,
    exclude_id: Option<i64>,
) -> Result<bool, DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let mut select = posts::Entity::find()
        .filter(posts::Column::TenantId.eq(tenant_id))
        .filter(posts::Column::SiteId.eq(site_id))
        .filter(posts::Column::PostType.eq(post_type))
        .filter(posts::Column::Slug.eq(slug));
    if let Some(id) = exclude_id {
        select = select.filter(posts::Column::Id.ne(id));
    }
    let found = select.one(&txn).await?.is_some();
    txn.commit().await?;
    Ok(found)
}

pub async fn list_revisions(
    db: &DatabaseConnection,
    tenant_id: i64,
    post_id: i64,
) -> Result<Vec<post_revisions::Model>, DbErr> {
    let txn = db.begin().await?;
    bind_tenant(&txn, tenant_id).await?;
    let rows = post_revisions::Entity::find()
        .filter(post_revisions::Column::TenantId.eq(tenant_id))
        .filter(post_revisions::Column::PostId.eq(post_id))
        .order_by_desc(post_revisions::Column::CreatedAt)
        .all(&txn)
        .await?;
    txn.commit().await?;
    Ok(rows)
}

async fn load_detail(txn: &DatabaseTransaction, post: posts::Model) -> Result<PostDetail, DbErr> {
    let translations = post_translations::Entity::find()
        .filter(post_translations::Column::PostId.eq(post.id))
        .all(txn)
        .await?;
    let term_taxonomy_ids = post_terms::Entity::find()
        .filter(post_terms::Column::PostId.eq(post.id))
        .all(txn)
        .await?
        .into_iter()
        .map(|row| row.term_taxonomy_id)
        .collect();
    Ok(PostDetail {
        post,
        translations,
        term_taxonomy_ids,
    })
}

async fn write_translations(
    txn: &DatabaseTransaction,
    tenant_id: i64,
    post_id: i64,
    translations: &[NewTranslation],
) -> Result<(), DbErr> {
    for tr in translations {
        post_translations::ActiveModel {
            post_id: Set(post_id),
            tenant_id: Set(tenant_id),
            locale: Set(tr.locale.clone()),
            title: Set(tr.title.clone()),
            excerpt: Set(tr.excerpt.clone()),
            body_json: Set(tr.body_json.clone()),
            body_html: Set(tr.body_html.clone()),
            word_count: Set(tr.word_count),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        // 每次保存为该语言留一份版本快照
        post_revisions::ActiveModel {
            post_id: Set(post_id),
            tenant_id: Set(tenant_id),
            locale: Set(tr.locale.clone()),
            title: Set(tr.title.clone()),
            body_json: Set(tr.body_json.clone()),
            ..Default::default()
        }
        .insert(txn)
        .await?;
    }
    Ok(())
}

async fn write_terms(
    txn: &DatabaseTransaction,
    tenant_id: i64,
    post_id: i64,
    term_taxonomy_ids: &[i64],
) -> Result<(), DbErr> {
    for tt_id in term_taxonomy_ids {
        post_terms::ActiveModel {
            post_id: Set(post_id),
            term_taxonomy_id: Set(*tt_id),
            tenant_id: Set(tenant_id),
        }
        .insert(txn)
        .await?;
    }
    Ok(())
}
