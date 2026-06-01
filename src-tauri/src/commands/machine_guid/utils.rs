// 机器码工具函数

use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

// 预编译正则表达式
static UUID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
        .expect("Failed to compile UUID regex")
});
static HEX32_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9a-f]{32}$").expect("Failed to compile HEX32 regex"));

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn get_macos_override_path() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_default()
        .join(".kiro-account-manager")
        .join("machine-id-override")
}

pub fn generate_random_machine_id() -> String {
    Uuid::new_v4().to_string().to_lowercase()
}

pub fn is_valid_machine_id(id: &str) -> bool {
    let lower = id.to_lowercase();
    UUID_REGEX.is_match(&lower) || HEX32_REGEX.is_match(&lower)
}

#[cfg(target_os = "linux")]
pub fn format_as_uuid(hex: &str) -> String {
    let c = hex.replace("-", "").to_lowercase();
    if c.len() != 32 {
        return c;
    }
    format!(
        "{}-{}-{}-{}-{}",
        &c[0..8],
        &c[8..12],
        &c[12..16],
        &c[16..20],
        &c[20..32]
    )
}

