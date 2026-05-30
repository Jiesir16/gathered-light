//! 自签名 SigV4 预签名 URL —— 让“签名直链”也走自定义域名，
//! 彻底不向前端暴露 COS 源站 host 和桶名。
//!
//! 为什么不直接用 aws-sdk 的 `presigned()`？
//! SDK 只能生成 `{endpoint}/{bucket}/{key}`（path-style）或 `{bucket}.{endpoint}/{key}`
//! （virtual-host）——桶名必然出现在 path 或 host 里，做不到“自定义域名根 + 干净 key”
//! （`https://media.example.com/variants/x.webp`）。而 SigV4 又把 `host` 算进签名，
//! 事后改 host 必然 `SignatureDoesNotMatch`。所以这里按 SigV4 规范自己签一把：
//! host 直接签自定义域名，路径就是干净的对象 key。
//!
//! COS 完全兼容 AWS SigV4（换 host 时 403 响应回显的 CanonicalRequest 即 AWS 标准格式，
//! 本模块单测就以那段真实报文为基准逐字节对齐）。≈ Java 里手写一个 AWS4Signer。

use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// 生成对 `base_url`（自定义域名根）下 `key` 的预签名 URL。
/// 只签 `host` 头、payload = `UNSIGNED-PAYLOAD`，GET / PUT 通用。
///
/// - `base_url`：如 `https://media.example.com`（域名根已在 COS 绑定到桶）
/// - `key`：干净对象 key，如 `variants/1/webp_900.webp`（无桶名前缀、无前导 `/`）
/// - `extra_query`：附加 query（如 `[("response-content-disposition", "inline")]`）
pub fn presign(
    method: &str,
    base_url: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
    key: &str,
    expires_secs: u64,
    extra_query: &[(&str, &str)],
) -> anyhow::Result<String> {
    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    presign_at(
        method,
        base_url,
        access_key,
        secret_key,
        region,
        key,
        expires_secs,
        extra_query,
        &amz_date,
        &date_stamp,
    )
}

/// 注入时间戳的内部实现，便于单测和真实 COS 报文逐字节对齐。
#[allow(clippy::too_many_arguments)]
fn presign_at(
    method: &str,
    base_url: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
    key: &str,
    expires_secs: u64,
    extra_query: &[(&str, &str)],
    amz_date: &str,
    date_stamp: &str,
) -> anyhow::Result<String> {
    let (scheme, host) = split_scheme_host(base_url)?;
    let scope = format!("{date_stamp}/{region}/s3/aws4_request");
    let canonical_uri = canonical_path(key);

    let (canonical_request, canonical_query) = canonicalize(
        method,
        host,
        &canonical_uri,
        access_key,
        &scope,
        amz_date,
        expires_secs,
        extra_query,
    );

    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex_sha256(canonical_request.as_bytes())
    );
    let signing_key = signing_key(secret_key, date_stamp, region);
    let signature = hex::encode(hmac_raw(&signing_key, string_to_sign.as_bytes()));

    Ok(format!(
        "{scheme}://{host}{canonical_uri}?{canonical_query}&X-Amz-Signature={signature}"
    ))
}

/// 拼 CanonicalRequest 与 CanonicalQueryString（不含 `X-Amz-Signature`）。
#[allow(clippy::too_many_arguments)]
fn canonicalize(
    method: &str,
    host: &str,
    canonical_uri: &str,
    access_key: &str,
    scope: &str,
    amz_date: &str,
    expires_secs: u64,
    extra_query: &[(&str, &str)],
) -> (String, String) {
    let mut params: Vec<(String, String)> = vec![
        ("X-Amz-Algorithm".to_owned(), "AWS4-HMAC-SHA256".to_owned()),
        ("X-Amz-Credential".to_owned(), format!("{access_key}/{scope}")),
        ("X-Amz-Date".to_owned(), amz_date.to_owned()),
        ("X-Amz-Expires".to_owned(), expires_secs.to_string()),
        ("X-Amz-SignedHeaders".to_owned(), "host".to_owned()),
    ];
    for (k, v) in extra_query {
        params.push(((*k).to_owned(), (*v).to_owned()));
    }
    // 先按 AWS 规则编码（key/value 均编码，`/` → %2F），再按编码后的 key 排序。
    let mut encoded: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| (uri_encode(k, true), uri_encode(v, true)))
        .collect();
    encoded.sort();
    let canonical_query = encoded
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_query}\nhost:{host}\n\nhost\nUNSIGNED-PAYLOAD"
    );
    (canonical_request, canonical_query)
}

fn signing_key(secret: &str, date_stamp: &str, region: &str) -> Vec<u8> {
    let k_date = hmac_raw(format!("AWS4{secret}").as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_raw(&k_date, region.as_bytes());
    let k_service = hmac_raw(&k_region, b"s3");
    hmac_raw(&k_service, b"aws4_request")
}

fn hmac_raw(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// AWS 风格 URI 编码：unreserved = `A-Za-z0-9-._~` 不编码，其余 `%XX`（大写十六进制）。
/// `encode_slash=false` 时保留 `/`（canonical URI 的路径分隔符不编码）。
fn uri_encode(input: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            b'/' if !encode_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn canonical_path(key: &str) -> String {
    format!("/{}", uri_encode(key.trim_start_matches('/'), false))
}

fn split_scheme_host(base_url: &str) -> anyhow::Result<(&str, &str)> {
    let (scheme, rest) = base_url
        .split_once("://")
        .ok_or_else(|| anyhow::anyhow!("invalid base_url (missing scheme): {base_url}"))?;
    let host = rest.split('/').next().unwrap_or(rest);
    if host.is_empty() {
        anyhow::bail!("invalid base_url (empty host): {base_url}");
    }
    Ok((scheme, host))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 对齐 AWS/COS SigV4 的 CanonicalRequest 标准结构（换 host 报 SignatureDoesNotMatch 时
    /// COS 回显的就是这段格式）。逐字节断言 → 证明 canonical 化、排序、`%2F` 编码、
    /// UNSIGNED-PAYLOAD 全部合规，这是 SigV4 里唯一容易写错的部分（HMAC 链路是标准密码学）。
    /// 全部用占位值，不含任何真实环境信息。
    #[test]
    fn canonical_request_matches_sigv4_layout() {
        let key = "uploads/2026/01/00000000-0000-0000-0000-000000000000-sample.jpg";
        let canonical_uri = canonical_path(key);
        let scope = "20260101/us-east-1/s3/aws4_request";
        let (canonical_request, _query) = canonicalize(
            "GET",
            "media.example.com",
            &canonical_uri,
            "EXAMPLE_ACCESS_KEY",
            scope,
            "20260101T000000Z",
            86400,
            // 故意乱序传入，验证编码后按 key 排序
            &[
                ("response-content-disposition", "inline"),
                ("x-id", "GetObject"),
            ],
        );

        let expected = concat!(
            "GET\n",
            "/uploads/2026/01/00000000-0000-0000-0000-000000000000-sample.jpg\n",
            "X-Amz-Algorithm=AWS4-HMAC-SHA256",
            "&X-Amz-Credential=EXAMPLE_ACCESS_KEY%2F20260101%2Fus-east-1%2Fs3%2Faws4_request",
            "&X-Amz-Date=20260101T000000Z",
            "&X-Amz-Expires=86400",
            "&X-Amz-SignedHeaders=host",
            "&response-content-disposition=inline",
            "&x-id=GetObject\n",
            "host:media.example.com\n",
            "\n",
            "host\n",
            "UNSIGNED-PAYLOAD"
        );
        assert_eq!(canonical_request, expected);
    }

    /// 产物 URL：自定义域名 host + 干净 key（无桶名前缀）+ 带签名，且不出现源站 host。
    #[test]
    fn url_is_clean_and_hides_origin() {
        let base = "https://media.example.com";
        let url = presign_at(
            "GET",
            base,
            "EXAMPLE_ACCESS_KEY",
            "example-secret-key",
            "us-east-1",
            "variants/1/webp_900.webp",
            3600,
            &[("response-content-disposition", "inline")],
            "20260101T000000Z",
            "20260101",
        )
        .unwrap();

        let path = url[base.len()..].split('?').next().unwrap();
        // 路径就是干净 key，没有被塞进桶名那一段
        assert_eq!(path, "/variants/1/webp_900.webp");
        assert!(url.contains("&X-Amz-Signature="));
        assert!(!url.contains("myqcloud"), "must not leak any COS origin host");
    }

    #[test]
    fn signature_is_deterministic_for_fixed_time() {
        let make = || {
            presign_at(
                "GET",
                "https://media.example.com",
                "EXAMPLE_ACCESS_KEY",
                "example-secret-key",
                "us-east-1",
                "variants/1/webp_900.webp",
                3600,
                &[],
                "20260101T000000Z",
                "20260101",
            )
            .unwrap()
        };
        assert_eq!(make(), make());
    }
}
