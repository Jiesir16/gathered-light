use cms_api::{
    bootstrap, config, observability, repositories::photo_repo, services::photo_service,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::init();

    let cfg = config::load()?;
    let state = bootstrap::AppState::init(cfg).await?;
    photo_repo::truncate_all(&state.db).await?;
    let report = photo_service::seed_demo_photos(&state).await?;

    println!(
        "seeded {} photos (public={} locked={} private={})",
        report.total, report.public, report.locked, report.private
    );
    Ok(())
}
