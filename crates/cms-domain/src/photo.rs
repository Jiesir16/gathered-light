use serde::{Deserialize, Serialize};

use crate::DomainError;

/// 与前端原型 `design/data.jsx` 中 `privacy: "public" | "locked" | "private"` 严格对齐。
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Privacy {
    Public,
    Locked,
    Private,
}

impl Privacy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Locked => "locked",
            Self::Private => "private",
        }
    }
}

impl std::str::FromStr for Privacy {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Self::Public),
            "locked" => Ok(Self::Locked),
            "private" => Ok(Self::Private),
            other => Err(DomainError::InvalidPrivacy(other.into())),
        }
    }
}

/// 与前端 i18n key 对齐：`data.jsx` 里 `title.zh / title.en`。
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Zh,
    En,
}

impl Locale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }
}

impl std::str::FromStr for Locale {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "zh" | "zh-CN" | "zh-Hans" => Ok(Self::Zh),
            "en" | "en-US" | "en-GB" => Ok(Self::En),
            other => Err(DomainError::InvalidLocale(other.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn privacy_roundtrip() {
        for p in [Privacy::Public, Privacy::Locked, Privacy::Private] {
            assert_eq!(Privacy::from_str(p.as_str()).unwrap(), p);
        }
    }

    #[test]
    fn privacy_rejects_unknown() {
        assert!(Privacy::from_str("draft").is_err());
    }
}
