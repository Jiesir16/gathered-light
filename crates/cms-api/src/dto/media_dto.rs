use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct PresignReq {
    pub file_name: String,
    pub mime_type: String,
    pub byte_size: u64,
}

#[derive(Debug, Serialize)]
pub struct PresignResp {
    pub method: &'static str,
    pub upload_url: String,
    pub public_url: String,
    pub storage_key: String,
    pub headers: BTreeMap<String, String>,
    pub max_bytes: u64,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct CompleteReq {
    pub storage_key: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteResp {
    pub asset_id: i64,
    pub storage_key: String,
    pub public_url: String,
}

/// 「修复历史数据」结果汇总（admin 按钮触发的 legacy key 修复）。
#[derive(Debug, Serialize)]
pub struct RepairResp {
    /// COS 里搬到干净 key 的旧对象数。
    pub objects_copied: usize,
    /// storage_key 去掉桶名前缀的 media_assets 行数。
    pub assets_fixed: u64,
    /// storage_key 去掉桶名前缀的 media_variants 行数。
    pub variants_fixed: u64,
    /// 重刷 ACL 的照片数。
    pub photos_resynced: usize,
}
