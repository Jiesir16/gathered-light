//! TipTap / ProseMirror JSON → 安全 HTML 渲染 + 摘要 + 字数。
//!
//! 设计依据：docs/CMS_REFACTOR_PROPOSAL.md §3.2 / §3.3。
//! 流程：JSON --(自写 walker, 纯函数可单测)--> 原始 HTML --(ammonia 白名单)--> 安全 HTML。
//! 安全红线：**绝不**直接渲染前端传来的 HTML；正文一律经此清洗后再落库 body_html。

use ammonia::Builder;
use serde_json::Value;

use crate::error::{AppError, AppResult};

pub struct RenderedBody {
    /// 清洗后的安全 HTML，前台直接渲染。
    pub html: String,
    /// 由纯文本截取的摘要（无标签）。
    pub excerpt: String,
    /// 估算字数（CJK 按字、其余按词）。
    pub word_count: i32,
}

/// 把 TipTap 文档 JSON 渲染为安全 HTML。
pub fn render_body(doc: &Value) -> AppResult<RenderedBody> {
    if !doc.is_object() {
        return Err(AppError::Validation("body_json must be a TipTap document".into()));
    }

    let mut raw = String::new();
    match doc.get("content").and_then(Value::as_array) {
        Some(content) => {
            for node in content {
                render_node(node, &mut raw);
            }
        }
        // 容错：直接传单节点时也渲染
        None if doc.get("type").is_some() => render_node(doc, &mut raw),
        None => {}
    }

    // ammonia 白名单清洗：去脚本/事件处理器/危险协议，外链统一加 rel。
    let html = Builder::default()
        .link_rel(Some("noopener nofollow noreferrer"))
        .clean(&raw)
        .to_string();

    let plain = strip_tags(&html);
    Ok(RenderedBody {
        excerpt: make_excerpt(&plain, 160),
        word_count: count_words(&plain),
        html,
    })
}

fn render_children(node: &Value, out: &mut String) {
    if let Some(content) = node.get("content").and_then(Value::as_array) {
        for child in content {
            render_node(child, out);
        }
    }
}

fn render_node(node: &Value, out: &mut String) {
    match node.get("type").and_then(Value::as_str) {
        Some("paragraph") => wrap(node, "p", out),
        Some("heading") => {
            let level = node
                .pointer("/attrs/level")
                .and_then(Value::as_i64)
                .unwrap_or(2)
                .clamp(1, 6);
            out.push_str(&format!("<h{level}>"));
            render_children(node, out);
            out.push_str(&format!("</h{level}>"));
        }
        Some("bulletList") => wrap(node, "ul", out),
        Some("orderedList") => wrap(node, "ol", out),
        Some("listItem") => wrap(node, "li", out),
        Some("blockquote") => wrap(node, "blockquote", out),
        Some("codeBlock") => {
            out.push_str("<pre><code>");
            if let Some(content) = node.get("content").and_then(Value::as_array) {
                for child in content {
                    if let Some(text) = child.get("text").and_then(Value::as_str) {
                        out.push_str(&escape_html(text));
                    }
                }
            }
            out.push_str("</code></pre>");
        }
        Some("horizontalRule") => out.push_str("<hr>"),
        Some("hardBreak") => out.push_str("<br>"),
        Some("image") => {
            let src = node.pointer("/attrs/src").and_then(Value::as_str).unwrap_or("");
            if !src.is_empty() {
                let alt = node.pointer("/attrs/alt").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\">",
                    escape_attr(src),
                    escape_attr(alt)
                ));
            }
        }
        Some("text") => render_text(node, out),
        // 未知节点：仍渲染其子节点，避免吞掉内容
        _ => render_children(node, out),
    }
}

fn wrap(node: &Value, tag: &str, out: &mut String) {
    out.push('<');
    out.push_str(tag);
    out.push('>');
    render_children(node, out);
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

fn render_text(node: &Value, out: &mut String) {
    let text = node.get("text").and_then(Value::as_str).unwrap_or("");
    let mut open = String::new();
    let mut close = String::new();
    if let Some(marks) = node.get("marks").and_then(Value::as_array) {
        for mark in marks {
            match mark.get("type").and_then(Value::as_str) {
                Some("bold") => {
                    open.push_str("<strong>");
                    close.insert_str(0, "</strong>");
                }
                Some("italic") => {
                    open.push_str("<em>");
                    close.insert_str(0, "</em>");
                }
                Some("strike") => {
                    open.push_str("<s>");
                    close.insert_str(0, "</s>");
                }
                Some("code") => {
                    open.push_str("<code>");
                    close.insert_str(0, "</code>");
                }
                Some("link") => {
                    let href = mark.pointer("/attrs/href").and_then(Value::as_str).unwrap_or("#");
                    open.push_str(&format!("<a href=\"{}\">", escape_attr(href)));
                    close.insert_str(0, "</a>");
                }
                _ => {}
            }
        }
    }
    out.push_str(&open);
    out.push_str(&escape_html(text));
    out.push_str(&close);
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape_html(s).replace('"', "&quot;")
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn make_excerpt(plain: &str, max_chars: usize) -> String {
    let chars: Vec<char> = plain.chars().collect();
    if chars.len() <= max_chars {
        return plain.to_owned();
    }
    let mut s: String = chars[..max_chars].iter().collect();
    s.push('…');
    s
}

fn count_words(plain: &str) -> i32 {
    let cjk = plain.chars().filter(|c| is_cjk(*c)).count();
    let words = plain
        .split_whitespace()
        .filter(|w| !w.chars().all(is_cjk))
        .count();
    (cjk + words) as i32
}

fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3040..=0x30FF | 0xAC00..=0xD7AF
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_and_sanitizes() {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 },
                  "content": [{ "type": "text", "text": "标题" }] },
                { "type": "paragraph",
                  "content": [
                      { "type": "text", "text": "hello " },
                      { "type": "text", "text": "world", "marks": [{ "type": "bold" }] }
                  ] }
            ]
        });
        let r = render_body(&doc).unwrap();
        assert!(r.html.contains("<h2>标题</h2>"));
        assert!(r.html.contains("<strong>world</strong>"));
        assert!(r.word_count >= 2);
    }

    #[test]
    fn strips_script_injection() {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [
                    { "type": "text", "text": "<script>alert(1)</script>" }
                ] }
            ]
        });
        let r = render_body(&doc).unwrap();
        assert!(!r.html.contains("<script>"));
    }
}
