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
