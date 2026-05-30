use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettingsResp {
    pub theme: String,
    #[serde(default = "default_range")]
    pub range: String,
    pub hero: HeroCopy,
    #[serde(default = "BrandEffect::default_effect", rename = "brandEffect")]
    pub brand_effect: BrandEffect,
}

/// 前台首屏文案，可在管理端编辑。整体以一条 JSON 存进 site_settings 的 `hero` 键。
#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct HeroCopy {
    /// 小标题 / kicker
    #[validate(nested)]
    pub issue: HeroI18n,
    /// 大字标题（「那个大字」）
    #[validate(nested)]
    pub headline: HeroI18n,
    /// 打字机循环播放的多行文案
    #[validate(nested)]
    pub intro_lines: HeroLines,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct HeroI18n {
    #[validate(length(min = 1, max = 200), custom(function = "validate_single_line"))]
    pub zh: String,
    #[validate(length(min = 1, max = 200), custom(function = "validate_single_line"))]
    pub en: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct HeroLines {
    #[validate(length(min = 1, max = 5), custom(function = "validate_lines"))]
    pub zh: Vec<String>,
    #[validate(length(min = 1, max = 5), custom(function = "validate_lines"))]
    pub en: Vec<String>,
}

/// 顶部品牌「拾光集」三字的动态字内光源配置，整体以 JSON 存进 site_settings 的 `brand_effect` 键。
#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BrandEffect {
    #[validate(custom(function = "validate_hex_color"))]
    pub color: String,
    #[validate(range(min = 0.6, max = 2.4))]
    pub glow_size: f32,
    #[validate(range(min = 0.5, max = 1.8))]
    pub glow_depth: f32,
    #[validate(range(min = 0.5, max = 2.5))]
    pub drift_speed: f32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateThemeReq {
    #[validate(custom(function = "validate_theme"))]
    pub theme: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRangeReq {
    #[validate(custom(function = "validate_range"))]
    pub range: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateHeroReq {
    #[validate(nested)]
    #[serde(flatten)]
    pub hero: HeroCopy,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateBrandEffectReq {
    #[validate(nested)]
    #[serde(flatten)]
    pub brand_effect: BrandEffect,
}

pub fn validate_theme(value: &str) -> Result<(), ValidationError> {
    if ["warm", "cool", "bold"].contains(&value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid theme"))
    }
}

pub fn validate_range(value: &str) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if (4..=32).contains(&trimmed.chars().count())
        && !trimmed.contains('\n')
        && !trimmed.contains('\r')
    {
        Ok(())
    } else {
        Err(ValidationError::new("invalid range"))
    }
}

fn validate_single_line(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() || value.contains('\n') || value.contains('\r') {
        Err(ValidationError::new("invalid line"))
    } else {
        Ok(())
    }
}

fn validate_lines(value: &[String]) -> Result<(), ValidationError> {
    for line in value {
        let len = line.chars().count();
        if !(1..=200).contains(&len) {
            return Err(ValidationError::new("line out of range"));
        }
        validate_single_line(line)?;
    }
    Ok(())
}

fn validate_hex_color(value: &str) -> Result<(), ValidationError> {
    let Some(hex) = value.strip_prefix('#') else {
        return Err(ValidationError::new("invalid color"));
    };
    if matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid color"))
    }
}

fn default_range() -> String {
    "2025 - 2026".to_owned()
}

impl HeroCopy {
    /// 默认文案，与前端 i18n.ts 保持一致；DB 未写入 hero 键时回退到这里。
    pub fn default_copy() -> Self {
        Self {
            issue: HeroI18n {
                zh: "拾光为集 · 个人影像手记".to_owned(),
                en: "Field Notebook · Personal Archive".to_owned(),
            },
            headline: HeroI18n {
                zh: "拾起那些被光掠过的、缓慢而不起眼的寻常一刻。".to_owned(),
                en: "Quiet hours, borrowed light, and the small ordinary in between.".to_owned(),
            },
            intro_lines: HeroLines {
                zh: vec![
                    "街角、天气、窗台上的一杯茶，均被轻轻收录于此，留于多年以后翻看。".to_owned(),
                    "把日常里微弱的光、风声与停顿收好，等某个下午再慢慢翻开。".to_owned(),
                ],
                en: vec![
                    "Streets, weather, and slow afternoons of rooms, kept here to be wandered through later.".to_owned(),
                    "Small light, quiet pauses, and the weather of a day are saved for another afternoon.".to_owned(),
                ],
            },
        }
    }
}

impl BrandEffect {
    pub fn default_effect() -> Self {
        Self {
            color: "#fff3cf".to_owned(),
            glow_size: 0.9,
            glow_depth: 0.65,
            drift_speed: 1.25,
        }
    }
}
