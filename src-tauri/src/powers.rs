// Powers 管理（v0.10.32 registry-v2: ~/.kiro/powers/）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// installed.json 中的已安装 Power 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPowerEntry {
    pub name: String,
    #[serde(default)]
    pub registry_id: String,
    #[serde(default)]
    pub auto_installed: bool,
}

/// installed.json 文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPowersFile {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub installed_powers: Vec<InstalledPowerEntry>,
    #[serde(default)]
    pub dismissed_auto_installs: Vec<DismissedEntry>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DismissedEntry {
    pub name: String,
    #[serde(default)]
    pub registry_id: String,
}

impl Default for InstalledPowersFile {
    fn default() -> Self {
        Self {
            version: default_version(),
            installed_powers: vec![],
            dismissed_auto_installs: vec![],
        }
    }
}

/// POWER.md frontmatter
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PowerFrontMatter {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub display_name: String,
}

/// 前端展示用的 Power 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub keywords: Vec<String>,
    pub registry_id: String,
    pub auto_installed: bool,
    /// POWER.md 完整内容
    pub power_md: String,
    /// mcp.json 中定义的 MCP 服务器名列表
    pub mcp_servers: Vec<String>,
    /// steering 目录下的 .md 文件列表
    pub steering_files: Vec<String>,
    /// 目录总大小
    pub size: u64,
}

pub struct PowersManager;

impl PowersManager {
    /// ~/.kiro/powers/
    pub fn powers_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("powers"))
    }

    /// 读取 installed.json
    pub fn load_installed() -> Result<InstalledPowersFile, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let path = dir.join("installed.json");
        if !path.exists() {
            return Ok(InstalledPowersFile::default());
        }
        let content = fs::read_to_string(&path).map_err(|e| format!("读取 installed.json 失败: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("解析 installed.json 失败: {e}"))
    }

    /// 保存 installed.json
    pub fn save_installed(data: &InstalledPowersFile) -> Result<(), String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        fs::create_dir_all(&dir).ok();
        let content = serde_json::to_string_pretty(data).map_err(|e| format!("序列化失败: {e}"))?;
        fs::write(dir.join("installed.json"), content).map_err(|e| format!("写入失败: {e}"))
    }

    /// 解析 POWER.md frontmatter
    fn parse_power_md(content: &str) -> PowerFrontMatter {
        let re = regex::Regex::new(r"^---\n([\s\S]*?)\n---").ok();
        let fm_str = re.and_then(|r| r.captures(content).map(|c| c[1].to_string()));

        let mut fm = PowerFrontMatter::default();
        if let Some(s) = fm_str {
            if let Some(v) = Self::extract_field(&s, "name") { fm.name = v; }
            if let Some(v) = Self::extract_field(&s, "description") { fm.description = v; }
            if let Some(v) = Self::extract_field(&s, "author") { fm.author = v; }
            if let Some(v) = Self::extract_field(&s, "license") { fm.license = v; }
            if let Some(v) = Self::extract_field(&s, "displayName") { fm.display_name = v; }
            // keywords: [k1, k2]
            if let Some(kw) = regex::Regex::new(r"keywords:\s*\[([^\]]*)\]").ok()
                .and_then(|r| r.captures(&s).map(|c| c[1].to_string()))
            {
                fm.keywords = kw.split(',').map(|k| k.trim().trim_matches(|c| c == '"' || c == '\'').to_string()).filter(|k| !k.is_empty()).collect();
            }
        }
        fm
    }

    fn extract_field(s: &str, field: &str) -> Option<String> {
        let pattern = format!(r#"{}:\s*['"]?([^'"\n]+)['"]?"#, field);
        regex::Regex::new(&pattern).ok()
            .and_then(|r| r.captures(s).map(|c| c[1].trim().to_string()))
    }

    /// 获取 Power 安装目录中的 MCP 服务器名列表
    fn get_mcp_server_names(power_dir: &Path) -> Vec<String> {
        let mcp_path = power_dir.join("mcp.json");
        if !mcp_path.exists() {
            return vec![];
        }
        let content = fs::read_to_string(&mcp_path).unwrap_or_default();
        // mcp.json: { "mcpServers": { "name": {...}, ... } }
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
        match parsed {
            Ok(v) => {
                v.get("mcpServers")
                    .and_then(|s| s.as_object())
                    .map(|obj| obj.keys().cloned().collect())
                    .unwrap_or_default()
            }
            Err(_) => vec![],
        }
    }

    /// 获取 steering 目录下的 .md 文件名列表
    fn get_steering_files(power_dir: &Path) -> Vec<String> {
        let steering_dir = power_dir.join("steering");
        if !steering_dir.exists() {
            return vec![];
        }
        fs::read_dir(&steering_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_file())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 计算目录大小
    fn dir_size(dir: &PathBuf) -> u64 {
        if !dir.exists() { return 0; }
        fs::read_dir(dir).ok().map(|entries| {
            entries.filter_map(Result::ok).map(|e| {
                let p = e.path();
                if p.is_file() {
                    fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
                } else if p.is_dir() {
                    Self::dir_size(&p)
                } else { 0 }
            }).sum()
        }).unwrap_or(0)
    }

    fn validate_power_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Power 名称不能为空".to_string());
        }
        if name.contains('/') || name.contains('\\') {
            return Err("Power 名称不能包含路径分隔符".to_string());
        }
        if name.contains("..") {
            return Err("Power 名称不能包含 ..".to_string());
        }

        let path = Path::new(name);
        for comp in path.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err("Power 名称非法".to_string());
            }
        }
        Ok(())
    }

    fn validate_branch_name(branch: &str) -> Result<(), String> {
        if branch.is_empty() {
            return Ok(());
        }
        if branch.starts_with('-') {
            return Err("分支名非法".to_string());
        }
        if branch.contains('\0') || branch.contains(' ') || branch.contains('\t') || branch.contains('\n') || branch.contains('\r') {
            return Err("分支名非法".to_string());
        }
        if branch.contains("..") || branch.contains("~") || branch.contains("^") || branch.contains(":") || branch.contains('?') || branch.contains('*') || branch.contains("\\") {
            return Err("分支名非法".to_string());
        }
        if branch.ends_with('.') || branch.ends_with('/') || branch.ends_with(".lock") || branch.contains("@{") || branch.contains("//") {
            return Err("分支名非法".to_string());
        }
        Ok(())
    }

    fn validate_clone_url(url: &str) -> Result<(), String> {
        let https_url = Self::convert_to_https_url(url);
        let parsed = reqwest::Url::parse(&https_url)
            .map_err(|_| "仓库 URL 非法".to_string())?;

        if parsed.scheme() != "https" {
            return Err("仅允许 https 仓库地址".to_string());
        }

        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if host != "github.com" {
            return Err("仅允许 github.com 仓库地址".to_string());
        }

        let mut segs = parsed.path().trim_start_matches('/').split('/').filter(|s| !s.is_empty());
        let owner = segs.next().unwrap_or_default();
        let repo = segs.next().unwrap_or_default();
        if owner.is_empty() || repo.is_empty() {
            return Err("仓库地址必须包含 owner/repo".to_string());
        }

        Ok(())
    }

    fn safe_power_subdir(base_dir: &Path, name: &str) -> Result<PathBuf, String> {
        Self::validate_power_name(name)?;
        let candidate = base_dir.join(name);

        if !candidate.starts_with(base_dir) {
            return Err("非法路径".to_string());
        }

        Ok(candidate)
    }

    fn safe_path_in_repo(clone_path: &Path, path_in_repo: &str) -> Result<PathBuf, String> {
        if path_in_repo.is_empty() {
            return Ok(clone_path.to_path_buf());
        }

        let relative = Path::new(path_in_repo);
        if relative.is_absolute() {
            return Err("仓库内路径必须是相对路径".to_string());
        }

        for comp in relative.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err("仓库内路径非法".to_string());
            }
        }

        let candidate = clone_path.join(relative);
        if !candidate.starts_with(clone_path) {
            return Err("仓库内路径非法".to_string());
        }

        Ok(candidate)
    }

    /// 加载所有已安装 Power 的详细信息
    pub fn load_all() -> Result<Vec<PowerInfo>, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let installed_dir = dir.join("installed");
        let installed_file = Self::load_installed()?;

        // 建立 name -> entry 映射
        let entry_map: HashMap<String, &InstalledPowerEntry> = installed_file.installed_powers.iter()
            .map(|e| (e.name.clone(), e))
            .collect();

        let mut powers = vec![];

        if !installed_dir.exists() {
            return Ok(powers);
        }

        for entry in fs::read_dir(&installed_dir).map_err(|e| format!("读取 installed 目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if !path.is_dir() { continue; }

            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let power_md_path = path.join("POWER.md");
            let power_md = fs::read_to_string(&power_md_path).unwrap_or_default();
            let fm = Self::parse_power_md(&power_md);

            let installed_entry = entry_map.get(&name);

            powers.push(PowerInfo {
                display_name: if fm.display_name.is_empty() { fm.name.clone() } else { fm.display_name.clone() },
                description: fm.description,
                author: fm.author,
                license: fm.license,
                keywords: fm.keywords,
                registry_id: installed_entry.map_or_else(String::new, |e| e.registry_id.clone()),
                auto_installed: installed_entry.is_some_and(|e| e.auto_installed),
                power_md,
                mcp_servers: Self::get_mcp_server_names(&path),
                steering_files: Self::get_steering_files(&path),
                size: Self::dir_size(&path),
                name,
            });
        }

        powers.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(powers)
    }

    /// 获取单个 Power 详情
    pub fn load(name: &str) -> Result<PowerInfo, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let installed_base = dir.join("installed");
        let power_dir = Self::safe_power_subdir(&installed_base, name)?;
        if !power_dir.exists() {
            return Err(format!("Power 不存在: {name}"));
        }

        let installed_file = Self::load_installed()?;
        let installed_entry = installed_file.installed_powers.iter().find(|e| e.name == name);

        let power_md_path = power_dir.join("POWER.md");
        let power_md = fs::read_to_string(&power_md_path).unwrap_or_default();
        let fm = Self::parse_power_md(&power_md);

        Ok(PowerInfo {
            display_name: if fm.display_name.is_empty() { fm.name.clone() } else { fm.display_name.clone() },
            description: fm.description,
            author: fm.author,
            license: fm.license,
            keywords: fm.keywords,
            registry_id: installed_entry.map_or_else(String::new, |e| e.registry_id.clone()),
            auto_installed: installed_entry.is_some_and(|e| e.auto_installed),
            power_md,
            mcp_servers: Self::get_mcp_server_names(&power_dir),
            steering_files: Self::get_steering_files(&power_dir),
            size: Self::dir_size(&power_dir),
            name: name.to_string(),
        })
    }

    /// 安装推荐 Power（与 Kiro IDE 一致的安装流程）
    /// 1. git clone 到 ~/.kiro/powers/repos/<name>/
    /// 2. 只复制 POWER.md, mcp.json, steering/*.md 到 ~/.kiro/powers/installed/<name>/
    /// 3. 更新 installed.json
    pub fn install(name: &str, clone_url: &str, path_in_repo: &str, branch: &str) -> Result<(), String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;

        Self::validate_power_name(name)?;
        Self::validate_clone_url(clone_url)?;
        Self::validate_branch_name(branch)?;

        let installed_base = dir.join("installed");
        let repos_base = dir.join("repos");
        let install_path = Self::safe_power_subdir(&installed_base, name)?;

        if install_path.exists() {
            return Err(format!("Power 已存在: {name}"));
        }

        // 1) clone 到 repos/<name>（与 Kiro IDE 一致）
        let clone_path = Self::safe_power_subdir(&repos_base, name)?;
        // 清理旧的 clone
        if clone_path.exists() {
            let _ = fs::remove_dir_all(&clone_path);
        }

        let branch_arg = if branch.is_empty() { "main".to_string() } else { branch.to_string() };

        // 转换 SSH URL 为 HTTPS（与 Kiro 的 convertToHttpsUrl 一致）
        let https_url = Self::convert_to_https_url(clone_url);

        fs::create_dir_all(clone_path.parent().unwrap_or(&dir))
            .map_err(|e| format!("创建 repos 目录失败: {e}"))?;

        let output = std::process::Command::new("git")
            .args(["clone", "--depth", "1", "--single-branch", "--branch", &branch_arg, &https_url])
            .arg(&clone_path)
            .output()
            .map_err(|e| format!("执行 git clone 失败（请确保已安装 git）: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = fs::remove_dir_all(&clone_path);
            return Err(format!("git clone 失败: {stderr}"));
        }

        // 2) 确定源目录
        let source_path = Self::safe_path_in_repo(&clone_path, path_in_repo)?;

        if !source_path.exists() {
            let _ = fs::remove_dir_all(&clone_path);
            return Err(format!("仓库中未找到路径: {path_in_repo}"));
        }

        let clone_path_canonical = fs::canonicalize(&clone_path)
            .map_err(|e| format!("解析仓库目录失败: {e}"))?;
        let source_path_canonical = fs::canonicalize(&source_path)
            .map_err(|e| format!("解析仓库内路径失败: {e}"))?;

        if !source_path_canonical.starts_with(&clone_path_canonical) {
            let _ = fs::remove_dir_all(&clone_path);
            return Err("仓库内路径非法".to_string());
        }

        let source_path = source_path_canonical;

        // 3) 只复制允许的文件到 installed/<name>/（与 Kiro copyPowerFiles 一致）
        fs::create_dir_all(&install_path).map_err(|e| format!("创建安装目录失败: {e}"))?;

        // 复制 POWER.md 和 mcp.json
        for file in &["POWER.md", "mcp.json"] {
            let src = source_path.join(file);
            if src.exists() {
                fs::copy(&src, install_path.join(file))
                    .map_err(|e| format!("复制 {file} 失败: {e}"))?;
            }
        }

        // 复制 steering/ 目录（只复制 .md 文件）
        let steering_src = source_path.join("steering");
        if steering_src.exists() && steering_src.is_dir() {
            let steering_dst = install_path.join("steering");
            Self::copy_steering_dir(&steering_src, &steering_dst)?;
        }

        // 4) 更新 installed.json
        let mut installed = Self::load_installed()?;
        if !installed.installed_powers.iter().any(|e| e.name == name) {
            installed.installed_powers.push(InstalledPowerEntry {
                name: name.to_string(),
                registry_id: "kiro-recommended".to_string(),
                auto_installed: false,
            });
        }
        // 从 dismissed 列表中移除
        installed.dismissed_auto_installs.retain(|d| d.name != name);
        Self::save_installed(&installed)?;

        Ok(())
    }

    /// SSH URL 转 HTTPS URL
    fn convert_to_https_url(url: &str) -> String {
        // git@github.com:user/repo.git -> https://github.com/user/repo.git
        if url.starts_with("git@") {
            let s = url.strip_prefix("git@").unwrap_or(url);
            let s = s.replacen(':', "/", 1);
            return format!("https://{s}");
        }
        url.to_string()
    }

    /// 复制 steering 目录（只复制 .md 文件，递归子目录）
    fn copy_steering_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
        fs::create_dir_all(dst).map_err(|e| format!("创建 steering 目录失败: {e}"))?;
        for entry in fs::read_dir(src).map_err(|e| format!("读取 steering 目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            let metadata = fs::symlink_metadata(&src_path)
                .map_err(|e| format!("读取文件元信息失败: {e}"))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                Self::copy_steering_dir(&src_path, &dst_path)?;
            } else if metadata.is_file() && src_path.extension().is_some_and(|e| e == "md") {
                fs::copy(&src_path, &dst_path).map_err(|e| format!("复制文件失败: {e}"))?;
            }
        }
        Ok(())
    }

    /// 卸载 Power（删除目录 + 从 installed.json 中移除）
    pub fn uninstall(name: &str) -> Result<(), String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let installed_base = dir.join("installed");
        let power_dir = Self::safe_power_subdir(&installed_base, name)?;
        if power_dir.exists() {
            fs::remove_dir_all(&power_dir).map_err(|e| format!("删除 Power 目录失败: {e}"))?;
        }

        // 从 installed.json 移除
        let mut installed = Self::load_installed()?;
        installed.installed_powers.retain(|e| e.name != name);
        // 加入 dismissed 列表防止自动重装
        if !installed.dismissed_auto_installs.iter().any(|d| d.name == name) {
            installed.dismissed_auto_installs.push(DismissedEntry {
                name: name.to_string(),
                registry_id: String::new(),
            });
        }
        Self::save_installed(&installed)?;

        Ok(())
    }

    /// 获取注册表列表（registries/ 目录下的 .json 文件）
    pub fn list_registries() -> Result<Vec<RegistryInfo>, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let reg_dir = dir.join("registries");
        if !reg_dir.exists() {
            return Ok(vec![]);
        }

        let mut registries = vec![];
        for entry in fs::read_dir(&reg_dir).map_err(|e| format!("读取 registries 目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|e| e != "json") { continue; }

            let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let id = file_name.trim_end_matches(".json").to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();

            registries.push(RegistryInfo {
                id,
                name: parsed.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                registry_type: parsed.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                power_count: parsed.get("powers").and_then(|v| v.as_array()).map_or(0, |a| a.len()),
            });
        }
        Ok(registries)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryInfo {
    pub id: String,
    pub name: String,
    pub registry_type: String,
    pub power_count: usize,
}

/// 推荐 Power 条目（来自远程 registry）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedPower {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub repository_url: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub repository_clone_url: String,
    #[serde(default)]
    pub path_in_repo: String,
    #[serde(default)]
    pub repository_branch: String,
    /// 前端用: 是否已安装
    #[serde(default)]
    pub installed: bool,
}

/// 远程推荐 registry 响应
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedRegistryResponse {
    #[serde(default, rename = "schemaVersion")]
    pub _schema_version: String,
    #[serde(default)]
    pub powers: Vec<RecommendedPower>,
}

const RECOMMENDED_REGISTRY_URL: &str = "https://prod.download.desktop.kiro.dev/powers/default_registry.json";

impl PowersManager {
    /// 拉取推荐 Powers 列表，并标记已安装状态
    pub async fn fetch_recommended() -> Result<Vec<RecommendedPower>, String> {
        let resp = reqwest::get(RECOMMENDED_REGISTRY_URL)
            .await
            .map_err(|e| format!("请求推荐列表失败: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let mut registry: RecommendedRegistryResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析推荐列表失败: {e}"))?;

        // 标记已安装
        let installed_names: std::collections::HashSet<String> = Self::load_installed()
            .unwrap_or_default()
            .installed_powers
            .into_iter()
            .map(|e| e.name)
            .collect();

        // 也检查 installed/ 目录
        let installed_dir_names: std::collections::HashSet<String> = Self::powers_dir()
            .map(|d| d.join("installed"))
            .filter(|d| d.exists())
            .and_then(|d| fs::read_dir(&d).ok())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for power in &mut registry.powers {
            power.installed = installed_names.contains(&power.name) || installed_dir_names.contains(&power.name);
        }

        Ok(registry.powers)
    }
}
