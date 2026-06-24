//! 统一 Post 内容模型的领域枚举（对标 WordPress post_type / post_status）。
//!
//! 与 DB CHECK 约束严格对齐：
//!   posts.post_type ∈ {post, page, photo}
//!   posts.status    ∈ {draft, published, scheduled, private, trash}
//! 可见性沿用现有 `Privacy`（public/locked/private），不另立枚举。

use serde::{Deserialize, Serialize};

use crate::DomainError;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostType {
    Post,
    Page,
    Photo,
}

impl PostType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Post => "post",
            Self::Page => "page",
            Self::Photo => "photo",
        }
    }
}

impl std::str::FromStr for PostType {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "post" => Ok(Self::Post),
            "page" => Ok(Self::Page),
            "photo" => Ok(Self::Photo),
            other => Err(DomainError::InvalidPostType(other.into())),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    Published,
    Scheduled,
    Private,
    Trash,
}

impl PostStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Scheduled => "scheduled",
            Self::Private => "private",
            Self::Trash => "trash",
        }
    }
}

impl std::str::FromStr for PostStatus {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "scheduled" => Ok(Self::Scheduled),
            "private" => Ok(Self::Private),
            "trash" => Ok(Self::Trash),
            other => Err(DomainError::InvalidPostStatus(other.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn post_type_roundtrip() {
        for v in [PostType::Post, PostType::Page, PostType::Photo] {
            assert_eq!(PostType::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn post_status_roundtrip() {
        for v in [
            PostStatus::Draft,
            PostStatus::Published,
            PostStatus::Scheduled,
            PostStatus::Private,
            PostStatus::Trash,
        ] {
            assert_eq!(PostStatus::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn rejects_unknown() {
        assert!(PostType::from_str("video").is_err());
        assert!(PostStatus::from_str("archived").is_err());
    }
}
