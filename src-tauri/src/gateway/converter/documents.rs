//! 文档（PDF 等附件）处理：三协议入站形状 → Kiro `DocumentBlock`。
//!
//! 官方约束（extension.js Smithy schema 与发送前校验，2026-10-22 取证）：
//! - `DocumentSource` union 仅含 `bytes`（与 `ImageSource$` 同构），无 Files API
//!   file_id 通道，file_id 形态的入站部件只能跳过；
//! - 每条消息最多 5 个文档（`I6r=5`，`DOCUMENT_COUNT_EXCEEDED`）；
//! - 单条消息内按 `name` 去重（保留首个）。

use super::*;

/// 官方每消息文档数上限（extension.js `I6r=5`）。
const MAX_DOCUMENTS_PER_MESSAGE: usize = 5;
/// 与图片同口径的解码后大小上限，防止超大附件撑爆上游请求体。
const MAX_DOCUMENT_SOURCE_BYTES: usize = 5 * 1024 * 1024;

pub fn extract_documents(content: Option<&Value>) -> Vec<DocumentBlock> {
    let Some(Value::Array(items)) = content else {
        return Vec::new();
    };

    let mut documents = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    for item in items {
        let Some(document) = extract_document_block(item) else {
            continue;
        };
        if !seen_names.insert(document.name.clone()) {
            log::warn!(
                "[协议映射] 单条消息内同名文档去重（保留首个）: {}",
                document.name
            );
            continue;
        }
        documents.push(document);
    }
    documents
}

fn extract_document_block(item: &Value) -> Option<DocumentBlock> {
    match item.get("type").and_then(Value::as_str).unwrap_or_default() {
        // Anthropic Messages document 块
        "document" => anthropic_document_block(item),
        // OpenAI Chat file 部件（file 对象内嵌）
        "file" => file_descriptor_to_document(item.get("file")?),
        // OpenAI Responses input_file item（字段平铺）
        "input_file" => file_descriptor_to_document(item),
        _ => None,
    }
}

fn anthropic_document_block(item: &Value) -> Option<DocumentBlock> {
    let source = item.get("source")?;
    let source_type = source
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("base64");
    if source_type != "base64" {
        log::warn!(
            "[协议映射] document source.type=\"{source_type}\" 无 Kiro 对应通道（官方仅 bytes），已跳过"
        );
        return None;
    }
    let bytes = source.get("data").and_then(Value::as_str)?;
    let media_type = source.get("media_type").and_then(Value::as_str)?;
    let format = document_format_from_media_type(media_type)?;
    let name = item
        .get("title")
        .or_else(|| item.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("document.{format}"));
    make_document(name, format, bytes)
}

fn file_descriptor_to_document(file: &Value) -> Option<DocumentBlock> {
    if file.get("file_data").is_none() {
        log::warn!(
            "[协议映射] 文档缺少 file_data（file_id 形态无 Kiro 对应通道，官方仅 bytes），已跳过"
        );
        return None;
    }
    let file_data = file.get("file_data").and_then(Value::as_str)?;
    let (media_type, bytes) = parse_document_data_url(file_data)?;
    let filename = file
        .get("filename")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "document".to_string());
    let (name, ext_format) = split_filename(&filename);
    let format = document_format_from_media_type(&media_type)
        .or(ext_format)
        .filter(|format| is_known_document_format(format))?;
    make_document(name, format, &bytes)
}

fn make_document(name: String, format: String, bytes: &str) -> Option<DocumentBlock> {
    if encoded_document_exceeds_limit(bytes) {
        log::warn!(
            "[协议映射] 文档 {} 超过 {} MB 上限，已跳过",
            name,
            MAX_DOCUMENT_SOURCE_BYTES / (1024 * 1024)
        );
        return None;
    }
    Some(DocumentBlock {
        format,
        name,
        source: DocumentSource::Bytes {
            bytes: bytes.to_string(),
        },
    })
}

/// 官方 `QLr` 正向 mime 表的文档子集；未知类型返回 None（调用方跳过）。
fn document_format_from_media_type(media_type: &str) -> Option<String> {
    let format = match media_type.trim().to_ascii_lowercase().as_str() {
        "text/csv" => "csv",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "text/html" => "html",
        "text/markdown" => "md",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "image/gif" => "gif",
        "image/jpeg" => "jpeg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => return None,
    };
    Some(format.to_string())
}

fn parse_document_data_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("data:")?;
    let (meta, bytes) = rest.split_once(',')?;
    let media_type = meta.split(';').next().unwrap_or_default().to_string();
    Some((media_type, bytes.to_string()))
}

/// 扩展名兑底也必须落在官方支持列表内（`tfd`），未知格式跳过而非原样透传，
/// 避免整请求被上游拒收。含 mime 表没有的 `doc`/`jpg` 别名。
fn is_known_document_format(format: &str) -> bool {
    matches!(
        format,
        "csv" | "doc" | "docx" | "html" | "md" | "pdf" | "txt" | "xls" | "xlsx"
            | "gif" | "jpeg" | "jpg" | "png" | "webp"
    )
}

/// "report.pdf" → ("report", Some("pdf"))；无扩展名 → (原文, None)。
fn split_filename(filename: &str) -> (String, Option<String>) {
    match filename.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => (
            stem.to_string(),
            Some(ext.trim().to_ascii_lowercase()),
        ),
        _ => (filename.to_string(), None),
    }
}

fn encoded_document_exceeds_limit(bytes: &str) -> bool {
    // base64 长度 → 原始字节数的保守估算（与 images.rs 同口径）
    bytes.len().saturating_mul(3) / 4 > MAX_DOCUMENT_SOURCE_BYTES
}

/// 官方每消息上限校验（`DOCUMENT_COUNT_EXCEEDED`），错误文案与官方一致。
pub fn ensure_documents_within_limit(documents: &[DocumentBlock]) -> Result<(), String> {
    if documents.len() > MAX_DOCUMENTS_PER_MESSAGE {
        return Err(format!(
            "Too many documents attached ({}). Maximum is {} per message.",
            documents.len(),
            MAX_DOCUMENTS_PER_MESSAGE
        ));
    }
    Ok(())
}
