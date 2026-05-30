use std::{io::Cursor, time::Duration};

use aws_sdk_s3::types::ObjectCannedAcl;
use cms_entity::media_assets::Model;
use image::{ImageDecoder, ImageEncoder};

use crate::{
    bootstrap::AppState,
    repositories::{media_repo, photo_repo},
};

pub async fn run(state: AppState) {
    tracing::info!("🖼  image_processor worker started");
    loop {
        match tick(&state).await {
            Ok(processed) if processed == 0 => {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            Ok(processed) => tracing::debug!(processed, "image_processor tick"),
            Err(error) => {
                tracing::error!(error = ?error, "image_processor tick failed; backing off");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

#[tracing::instrument(skip(state), fields(asset_id))]
async fn tick(state: &AppState) -> anyhow::Result<usize> {
    let claimed = media_repo::claim_pending(&state.db, 5).await?;
    let processed = claimed.len();
    for asset in claimed {
        tracing::Span::current().record("asset_id", asset.id);
        if let Err(error) = process_one(state, &asset).await {
            tracing::error!(error = ?error, asset_id = asset.id, "process failed");
            if let Err(status_error) =
                media_repo::update_status(&state.db, asset.id, "failed").await
            {
                tracing::error!(
                    error = ?status_error,
                    asset_id = asset.id,
                    "mark media asset failed"
                );
            }
        } else {
            media_repo::update_status(&state.db, asset.id, "ready").await?;
        }
    }
    Ok(processed)
}

async fn process_one(state: &AppState, asset: &Model) -> anyhow::Result<()> {
    let object = state
        .s3
        .get_object()
        .bucket(&state.config.s3.bucket)
        .key(&asset.storage_key)
        .send()
        .await?;
    let bytes = object.body.collect().await?.into_bytes();

    let image = decode_with_orientation(bytes.as_ref())?;
    let (orig_w, orig_h) = (image.width(), image.height());

    // 看绑定的 photo 是否 public，决定 variants 的 ACL；orphan asset 默认 private
    let is_public = match photo_repo::find_privacy_by_primary_asset(&state.db, asset.id).await {
        Ok(Some(privacy)) => privacy == "public",
        Ok(None) => false,
        Err(error) => {
            tracing::warn!(error = ?error, asset_id = asset.id, "lookup photo privacy failed; default private");
            false
        }
    };
    let variants_acl = if is_public {
        ObjectCannedAcl::PublicRead
    } else {
        ObjectCannedAcl::Private
    };

    for variant in [
        Variant {
            name: "thumb_400",
            long_side: 400,
            fmt: Fmt::Jpeg(75),
        },
        Variant {
            name: "medium_900",
            long_side: 900,
            fmt: Fmt::Jpeg(80),
        },
        Variant {
            name: "full_1800",
            long_side: 1800,
            fmt: Fmt::Jpeg(85),
        },
        Variant {
            name: "webp_900",
            long_side: 900,
            fmt: Fmt::Webp(80),
        },
    ] {
        let resized = resize_keep_ratio(&image, variant.long_side);
        let (width, height) = (resized.width(), resized.height());
        let (encoded, ext) = encode(resized, &variant.fmt)?;
        let key = format!("variants/{}/{}.{}", asset.id, variant.name, ext);

        state
            .s3
            .put_object()
            .bucket(&state.config.s3.bucket)
            .key(&key)
            .content_type(variant.fmt.mime())
            .content_disposition("inline")
            .acl(variants_acl.clone())
            .body(encoded.into())
            .send()
            .await?;

        media_repo::insert_variant(
            &state.db,
            asset.id,
            variant.name,
            &key,
            width as i32,
            height as i32,
        )
        .await?;
    }

    tracing::info!(
        asset_id = asset.id,
        orig_w,
        orig_h,
        is_public,
        "variants generated"
    );
    Ok(())
}

fn decode_with_orientation(bytes: &[u8]) -> anyhow::Result<image::DynamicImage> {
    let reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

struct Variant {
    name: &'static str,
    long_side: u32,
    fmt: Fmt,
}

enum Fmt {
    Jpeg(u8),
    Webp(u8),
}

impl Fmt {
    fn mime(&self) -> &'static str {
        match self {
            Fmt::Jpeg(_) => "image/jpeg",
            Fmt::Webp(_) => "image/webp",
        }
    }
}

fn resize_keep_ratio(img: &image::DynamicImage, long_side: u32) -> image::DynamicImage {
    let (width, height) = (img.width(), img.height());
    if width.max(height) <= long_side {
        return img.clone();
    }
    img.resize(long_side, long_side, image::imageops::FilterType::Lanczos3)
}

fn encode(img: image::DynamicImage, fmt: &Fmt) -> anyhow::Result<(Vec<u8>, &'static str)> {
    let mut buf = Vec::new();
    match fmt {
        Fmt::Jpeg(quality) => {
            let rgb = img.to_rgb8();
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, *quality);
            encoder.encode(
                &rgb,
                rgb.width(),
                rgb.height(),
                image::ColorType::Rgb8.into(),
            )?;
            Ok((buf, "jpg"))
        }
        Fmt::Webp(_quality) => {
            let rgba = img.to_rgba8();
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
            encoder.write_image(
                &rgba,
                rgba.width(),
                rgba.height(),
                image::ColorType::Rgba8.into(),
            )?;
            Ok((buf, "webp"))
        }
    }
}
