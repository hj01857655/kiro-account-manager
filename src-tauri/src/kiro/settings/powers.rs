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

impl PowerFrontMatter {
    /// Agent Plugin 布局回退：POWER.md 缺失时从 plugin.json 补元信息
    /// （bundle X3u 解析的 name/displayName/description/author/license/keywords）。
    /// POWER.md 已有值优先，plugin.json 只补空位。
    fn merged_with_plugin_json(self, plugin_json: &str) -> Self {
        let Ok(serde_json::Value::Object(map)) =
            serde_json::from_str::<serde_json::Value>(plugin_json)
        else {
            return self;
        };
        let read_str = |key: &str| -> Option<String> {
            map.get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        };
        let read_keywords = || -> Vec<String> {
            map.get("keywords")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut merged = self;
        if merged.name.is_empty() {
            merged.name = read_str("name").unwrap_or_default();
        }
        if merged.display_name.is_empty() {
            merged.display_name = read_str("displayName").unwrap_or_default();
        }
        if merged.description.is_empty() {
            merged.description = read_str("description").unwrap_or_default();
        }
        if merged.author.is_empty() {
            merged.author = map
                .get("author")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or_default();
        }
        if merged.license.is_empty() {
            merged.license = read_str("license").unwrap_or_default();
        }
        if merged.keywords.is_empty() {
            merged.keywords = read_keywords();
        }
        merged
    }
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
        let content =
            fs::read_to_string(&path).map_err(|e| format!("读取 installed.json 失败: {e}"))?;
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
            if let Some(v) = Self::extract_field(&s, "name") {
                fm.name = v;
            }
            if let Some(v) = Self::extract_field(&s, "description") {
                fm.description = v;
            }
            if let Some(v) = Self::extract_field(&s, "author") {
                fm.author = v;
            }
            if let Some(v) = Self::extract_field(&s, "license") {
                fm.license = v;
            }
            if let Some(v) = Self::extract_field(&s, "displayName") {
                fm.display_name = v;
            }
            // keywords: [k1, k2]
            if let Some(kw) = regex::Regex::new(r"keywords:\s*\[([^\]]*)\]")
                .ok()
                .and_then(|r| r.captures(&s).map(|c| c[1].to_string()))
            {
                fm.keywords = kw
                    .split(',')
                    .map(|k| k.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                    .filter(|k| !k.is_empty())
                    .collect();
            }
        }
        fm
    }

    fn extract_field(s: &str, field: &str) -> Option<String> {
        let pattern = format!(r#"{}:\s*['"]?([^'"\n]+)['"]?"#, field);
        regex::Regex::new(&pattern)
            .ok()
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
            Ok(v) => v
                .get("mcpServers")
                .and_then(|s| s.as_object())
                .map(|obj| obj.keys().cloned().collect())
                .unwrap_or_default(),
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
        if !dir.exists() {
            return 0;
        }
        fs::read_dir(dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| {
                        let p = e.path();
                        if p.is_file() {
                            fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
                        } else if p.is_dir() {
                            Self::dir_size(&p)
                        } else {
                            0
                        }
                    })
                    .sum()
            })
            .unwrap_or(0)
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
        if branch.contains('\0')
            || branch.contains(' ')
            || branch.contains('\t')
            || branch.contains('\n')
            || branch.contains('\r')
        {
            return Err("分支名非法".to_string());
        }
        if branch.contains("..")
            || branch.contains("~")
            || branch.contains("^")
            || branch.contains(":")
            || branch.contains('?')
            || branch.contains('*')
            || branch.contains("\\")
        {
            return Err("分支名非法".to_string());
        }
        if branch.ends_with('.')
            || branch.ends_with('/')
            || branch.ends_with(".lock")
            || branch.contains("@{")
            || branch.contains("//")
        {
            return Err("分支名非法".to_string());
        }
        Ok(())
    }

    fn validate_clone_url(url: &str) -> Result<(), String> {
        let https_url = Self::convert_to_https_url(url);
        let parsed = reqwest::Url::parse(&https_url).map_err(|_| "仓库 URL 非法".to_string())?;

        if parsed.scheme() != "https" {
            return Err("仅允许 https 仓库地址".to_string());
        }

        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if host != "github.com" {
            return Err("仅允许 github.com 仓库地址".to_string());
        }

        let mut segs = parsed
            .path()
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty());
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
        let entry_map: HashMap<String, &InstalledPowerEntry> = installed_file
            .installed_powers
            .iter()
            .map(|e| (e.name.clone(), e))
            .collect();

        let mut powers = vec![];

        if !installed_dir.exists() {
            return Ok(powers);
        }

        for entry in
            fs::read_dir(&installed_dir).map_err(|e| format!("读取 installed 目录失败: {e}"))?
        {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // 两种布局（对齐 1.1.70 bundle PowersManager）：
            // - 传统 Power：POWER.md frontmatter 提供元信息
            // - Agent Plugin（ULi 判定：目录含 plugin.json）：元信息在 plugin.json
            //   的 name/displayName/description，文档在 dev.kiro/INSTRUCTIONS.md
            let is_agent_plugin = path.join("plugin.json").is_file();
            let power_md_path = path.join("POWER.md");
            let power_md = fs::read_to_string(&power_md_path).unwrap_or_default();
            let fm = if is_agent_plugin {
                Self::parse_power_md(&power_md)
                    .merged_with_plugin_json(&fs::read_to_string(path.join("plugin.json"))
                        .unwrap_or_default())
            } else {
                Self::parse_power_md(&power_md)
            };

            let installed_entry = entry_map.get(&name);

            powers.push(PowerInfo {
                display_name: if fm.display_name.is_empty() {
                    fm.name.clone()
                } else {
                    fm.display_name.clone()
                },
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
        let installed_entry = installed_file
            .installed_powers
            .iter()
            .find(|e| e.name == name);

        let power_md_path = power_dir.join("POWER.md");
        let power_md = fs::read_to_string(&power_md_path).unwrap_or_default();
        let fm = Self::parse_power_md(&power_md);

        Ok(PowerInfo {
            display_name: if fm.display_name.is_empty() {
                fm.name.clone()
            } else {
                fm.display_name.clone()
            },
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
    /// 安装来自 git 仓库的 Power（推荐源 / GitHub URL 导入）。
    /// 1. git clone 到 ~/.kiro/powers/repos/<name>/
    /// 2. 只复制 POWER.md, plugin.json, mcp.json, steering/*.md 到 ~/.kiro/powers/installed/<name>/
    /// 3. 更新 installed.json
    ///
    /// `registry_id` 决定 installed.json 中记录的来源注册表：
    /// 推荐源为 `kiro-recommended`；用户自建为 `user-added`（见 `install_from_repo`）。
    pub fn install(
        name: &str,
        clone_url: &str,
        path_in_repo: &str,
        branch: &str,
        registry_id: &str,
    ) -> Result<(), String> {
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

        let branch_arg = if branch.is_empty() {
            "main".to_string()
        } else {
            branch.to_string()
        };

        // 转换 SSH URL 为 HTTPS（与 Kiro 的 convertToHttpsUrl 一致）
        let https_url = Self::convert_to_https_url(clone_url);

        fs::create_dir_all(clone_path.parent().unwrap_or(&dir))
            .map_err(|e| format!("创建 repos 目录失败: {e}"))?;

        let output = std::process::Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--single-branch",
                "--branch",
                &branch_arg,
                &https_url,
            ])
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

        let clone_path_canonical =
            fs::canonicalize(&clone_path).map_err(|e| format!("解析仓库目录失败: {e}"))?;
        let source_path_canonical =
            fs::canonicalize(&source_path).map_err(|e| format!("解析仓库内路径失败: {e}"))?;

        if !source_path_canonical.starts_with(&clone_path_canonical) {
            let _ = fs::remove_dir_all(&clone_path);
            return Err("仓库内路径非法".to_string());
        }

        let source_path = source_path_canonical;

        // 3) 只复制允许的文件到 installed/<name>/（与 Kiro copyPowerFiles 一致）
        //    与本地导入共用同一复制器，保证两种来源产出结构一致（含 plugin.json 支持）
        fs::create_dir_all(&install_path).map_err(|e| format!("创建安装目录失败: {e}"))?;
        if let Err(e) = Self::copy_power_files(&source_path, &install_path) {
            let _ = fs::remove_dir_all(&install_path);
            return Err(e);
        }

        // 4) 更新 installed.json
        let mut installed = Self::load_installed()?;
        if !installed.installed_powers.iter().any(|e| e.name == name) {
            installed.installed_powers.push(InstalledPowerEntry {
                name: name.to_string(),
                registry_id: registry_id.to_string(),
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

    /// 从公开 GitHub URL 导入自定义 Power（对应 Kiro `addCustomPowerByUrl`）。
    ///
    /// `url` 支持形式：
    /// - `https://github.com/<owner>/<repo>`
    /// - `https://github.com/<owner>/<repo>/tree/<branch>/<sub/dir>`
    ///
    /// 名称推导与 Kiro 一致：有子目录取子目录末段，否则取 repo 名，再 sanitize。
    /// 安装成功后写入 `registries/user-added.json` 与 `installed.json`。
    pub fn install_from_github_url(url: &str) -> Result<String, String> {
        let (clone_url, branch, path_in_repo, name) = Self::parse_github_url(url)?;

        Self::validate_power_name(&name)?;
        Self::validate_clone_url(&clone_url)?;
        Self::validate_branch_name(&branch)?;

        // 先安装（内部会 clone + 复制 + 写 installed.json）
        Self::install(
            &name,
            &clone_url,
            &path_in_repo,
            &branch,
            REGISTRY_ID_USER_ADDED,
        )?;

        // 再登记 user-added 注册表；失败则回滚已安装产物，避免出现"装了但 Kiro 不认识"
        let entry = UserAddedPowerEntry {
            name: name.clone(),
            description: format!("Custom power from {url}"),
            repository_url: Some(url.to_string()),
            source: PowerSource::Repo {
                repository_clone_url: clone_url,
                path_in_repo,
                repository_branch: branch,
            },
        };
        if let Err(e) = Self::upsert_user_added_entry(entry) {
            let _ = Self::uninstall(&name);
            return Err(format!("写入 user-added 注册表失败: {e}"));
        }

        Ok(name)
    }

    /// 解析 GitHub URL，返回 (clone_url, branch, path_in_repo, power_name)
    fn parse_github_url(url: &str) -> Result<(String, String, String, String), String> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err("URL 不能为空".to_string());
        }

        let normalized = Self::convert_to_https_url(trimmed);
        let without_scheme = normalized
            .strip_prefix("https://")
            .or_else(|| normalized.strip_prefix("http://"))
            .unwrap_or(&normalized);

        let mut segments = without_scheme.split('/').filter(|s| !s.is_empty());
        let host = segments.next().unwrap_or_default().to_ascii_lowercase();
        if host != "github.com" && host != "www.github.com" {
            return Err("仅支持 github.com 上的公开仓库".to_string());
        }

        let owner = segments.next().unwrap_or_default().to_string();
        let repo_raw = segments.next().unwrap_or_default().to_string();
        if owner.is_empty() || repo_raw.is_empty() {
            return Err("URL 必须包含 owner/repo".to_string());
        }
        let repo = repo_raw.trim_end_matches(".git").to_string();

        // 剩余段：可能形如 tree/<branch>/<sub/dir...>
        let rest: Vec<&str> = segments.collect();
        let (branch, path_in_repo) = if rest.first() == Some(&"tree") && rest.len() >= 2 {
            let b = rest[1].to_string();
            let p = rest[2..].join("/");
            (b, p)
        } else {
            (String::new(), String::new())
        };

        // 名称推导：有子目录取末段，否则取 repo 名
        let raw_name = if path_in_repo.is_empty() {
            repo.clone()
        } else {
            path_in_repo
                .rsplit('/')
                .next()
                .unwrap_or(&repo)
                .to_string()
        };
        let name: String = raw_name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
            .collect();
        let name = name.trim_matches('-').to_string();
        if name.is_empty() {
            return Err("无法从 URL 推导出合法的 Power 名称".to_string());
        }

        let clone_url = format!("https://github.com/{owner}/{repo}.git");
        Ok((clone_url, branch, path_in_repo, name))
    }

    /// 安装本地文件夹中的 Power（对应 Kiro `addCustomPowerByFolder`）。
    ///
    /// 流程（与 Kiro 一致）：
    /// 1. 校验源目录是合法 Power（含 `plugin.json` 或 `POWER.md`）；
    /// 2. 名称取目录名 sanitize 后的结果；目标已存在则视为已安装并跳过；
    /// 3. 复制清单文件 + `steering/` 下的 `.md` 到 `~/.kiro/powers/installed/<name>/`；
    /// 4. 登记到 `registries/user-added.json`（registryId = "user-added"）；
    /// 5. 写入 `installed.json`；任一步失败则回滚注册表与安装目录。
    ///
    /// 返回实际安装使用的 Power 名称（可能因 sanitize 与目录名不同）。
    pub fn install_from_local(source_dir: &str) -> Result<String, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let source = Path::new(source_dir);

        Self::validate_power_dir(source)?;

        let name = Self::derive_power_name(source)?;
        Self::validate_power_name(&name)?;

        let installed_base = dir.join("installed");
        let install_path = Self::safe_power_subdir(&installed_base, &name)?;

        if install_path.exists() {
            // 与 Kiro 一致：已安装则提示而非覆盖，避免抹掉用户改动
            return Err(format!("Power 已安装: {name}"));
        }

        // 源目录必须在安装目录之外，避免自拷贝导致递归
        if let (Ok(src_canon), Ok(powers_canon)) = (fs::canonicalize(source), fs::canonicalize(&dir))
        {
            if src_canon.starts_with(&powers_canon) {
                return Err("不能从 Kiro 的 powers 目录内部导入 Power".to_string());
            }
        }

        // 1) 复制文件
        fs::create_dir_all(&install_path).map_err(|e| format!("创建安装目录失败: {e}"))?;
        if let Err(e) = Self::copy_power_files(source, &install_path) {
            let _ = fs::remove_dir_all(&install_path);
            return Err(e);
        }

        // 2) 登记 user-added 注册表；失败则回滚已复制的目录
        let entry = UserAddedPowerEntry {
            name: name.clone(),
            description: format!("Custom power from {}", source.display()),
            repository_url: None,
            source: PowerSource::Local {
                path: source.to_string_lossy().to_string(),
            },
        };
        if let Err(e) = Self::upsert_user_added_entry(entry) {
            let _ = fs::remove_dir_all(&install_path);
            return Err(format!("写入 user-added 注册表失败: {e}"));
        }

        // 3) 写入 installed.json
        let mut installed = Self::load_installed()?;
        if !installed.installed_powers.iter().any(|e| e.name == name) {
            installed.installed_powers.push(InstalledPowerEntry {
                name: name.clone(),
                registry_id: REGISTRY_ID_USER_ADDED.to_string(),
                auto_installed: false,
            });
        }
        installed.dismissed_auto_installs.retain(|d| d.name != name);
        if let Err(e) = Self::save_installed(&installed) {
            let _ = fs::remove_dir_all(&install_path);
            let _ = Self::remove_user_added_entry(&name);
            return Err(format!("更新 installed.json 失败: {e}"));
        }

        Ok(name)
    }

    /// 复制 Power 文件到安装目录（对应 Kiro `copyPowerFiles`）。
    ///
    /// 只复制 Kiro 认可的载荷，不整目录搬运：
    /// - 根目录清单文件：`POWER.md`、`plugin.json`、`mcp.json`
    /// - `steering/` 目录下的全部 `.md`（递归，跳过符号链接）
    ///
    /// 不复制 `.git`、`README.md`、`LICENSE` 等仓库元数据——Kiro 亦不复制。
    fn copy_power_files(source: &Path, target: &Path) -> Result<(), String> {
        const ROOT_MANIFESTS: &[&str] = &["POWER.md", "plugin.json", "mcp.json"];

        for file in ROOT_MANIFESTS {
            let src = source.join(file);
            if src.is_file() {
                fs::copy(&src, target.join(file))
                    .map_err(|e| format!("复制 {file} 失败: {e}"))?;
            }
        }

        // steering/ 递归复制 .md
        let steering_src = source.join("steering");
        if steering_src.is_dir() {
            Self::copy_md_dir_recursive(&steering_src, &target.join("steering"))?;
        }

        // plugin.json 形态的 Power 可能把内容放在其他子目录，按 Kiro 语义仅补充 .md
        for sub in &["instructions", "docs"] {
            let sub_src = source.join(sub);
            if sub_src.is_dir() {
                Self::copy_md_dir_recursive(&sub_src, &target.join(sub))?;
            }
        }

        Ok(())
    }

    /// 递归复制目录下的 `.md` 文件（跳过符号链接与复制目录，避免逃逸）
    fn copy_md_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
        fs::create_dir_all(dst).map_err(|e| format!("创建目录失败: {e}"))?;
        for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            let metadata =
                fs::symlink_metadata(&src_path).map_err(|e| format!("读取文件元信息失败: {e}"))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                Self::copy_md_dir_recursive(&src_path, &dst_path)?;
            } else if metadata.is_file() && src_path.extension().is_some_and(|e| e == "md") {
                fs::copy(&src_path, &dst_path).map_err(|e| format!("复制文件失败: {e}"))?;
            }
        }
        Ok(())
    }

    /// 卸载 Power（删除目录 + 从 installed.json / user-added 注册表中移除）
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
        if !installed
            .dismissed_auto_installs
            .iter()
            .any(|d| d.name == name)
        {
            installed.dismissed_auto_installs.push(DismissedEntry {
                name: name.to_string(),
                registry_id: String::new(),
            });
        }
        Self::save_installed(&installed)?;

        // 若来自自定义来源，同步清理 user-added 注册表，避免残留失效条目
        // （忽略错误：注册表不存在或未登记该条目都属正常情形）
        let _ = Self::remove_user_added_entry(name);

        Ok(())
    }

    /// 获取注册表列表（registries/ 目录下的 .json 文件）。
    ///
    /// 跳过 `user-added.json`：它承载的是用户自建来源（本地/GitHub），
    /// 在 Kiro 中作为独立的 "Custom Powers" 展示，而非一个可浏览的注册表。
    pub fn list_registries() -> Result<Vec<RegistryInfo>, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let reg_dir = dir.join("registries");
        if !reg_dir.exists() {
            return Ok(vec![]);
        }

        let mut registries = vec![];
        for entry in fs::read_dir(&reg_dir).map_err(|e| format!("读取 registries 目录失败: {e}"))?
        {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|e| e != "json") {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if file_name == "user-added.json" {
                continue;
            }
            let id = file_name.trim_end_matches(".json").to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();

            registries.push(RegistryInfo {
                id,
                name: parsed
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                registry_type: parsed
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                power_count: parsed
                    .get("powers")
                    .and_then(|v| v.as_array())
                    .map_or(0, |a| a.len()),
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

const RECOMMENDED_REGISTRY_URL: &str =
    "https://prod.download.desktop.kiro.dev/powers/default_registry.json";

/// Kiro 内置注册表 ID 常量（与 IDE 实现一致）
pub(crate) const REGISTRY_ID_RECOMMENDED: &str = "kiro-recommended";
const REGISTRY_ID_USER_ADDED: &str = "user-added";

/// Power 来源：本地文件夹 或 git 仓库。
/// 对应 Kiro `addCustomPower` 的两条导入路径（folder / url）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PowerSource {
    /// 本地文件夹来源：`{ "type": "local", "path": "<绝对路径>" }`
    #[serde(rename = "local")]
    Local { path: String },
    /// git 仓库来源：`{ "type": "repo", "repositoryCloneUrl": ..., "pathInRepo": ..., "repositoryBranch": ... }`
    #[serde(rename = "repo")]
    Repo {
        #[serde(default, rename = "repositoryCloneUrl")]
        repository_clone_url: String,
        #[serde(default, rename = "pathInRepo")]
        path_in_repo: String,
        #[serde(default, rename = "repositoryBranch")]
        repository_branch: String,
    },
}

/// `registries/user-added.json` 中的自定义 Power 条目。
/// 字段名遵循 Kiro registry schema：name / description / source（+ 可选 repositoryUrl）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAddedPowerEntry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "repositoryUrl", skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,
    /// 来源信息；缺失时退化为「未知本地来源」而非让整表解析失败
    #[serde(default)]
    pub source: PowerSource,
}

impl Default for PowerSource {
    fn default() -> Self {
        PowerSource::Local {
            path: String::new(),
        }
    }
}

/// `registries/user-added.json` 文件结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserAddedRegistry {
    #[serde(default)]
    pub powers: Vec<UserAddedPowerEntry>,
}

impl PowersManager {
    /// `~/.kiro/powers/registries/user-added.json`
    ///
    /// 仅用于记录**自定义**（本地文件夹 / GitHub URL）Power 的来源，
    /// 以便 Kiro 侧能识别并支持「检查更新」。内置推荐源走 registry.json。
    pub fn user_added_registry_path() -> Result<PathBuf, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        Ok(dir.join("registries").join("user-added.json"))
    }

    /// 读取 user-added 注册表（不存在或损坏时返回空表，与 Kiro 行为一致）
    pub fn load_user_added_registry() -> Result<UserAddedRegistry, String> {
        let path = Self::user_added_registry_path()?;
        if !path.exists() {
            return Ok(UserAddedRegistry::default());
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Ok(UserAddedRegistry::default()),
        };
        // Kiro 在解析失败时打 warn 并新建空表，而非抛错
        Ok(serde_json::from_str(&content).unwrap_or_default())
    }

    /// 原子写入 user-added 注册表（先写 .tmp 再 rename，与 Kiro 实现一致）
    pub fn save_user_added_registry(registry: &UserAddedRegistry) -> Result<(), String> {
        let path = Self::user_added_registry_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建 registries 目录失败: {e}"))?;
        }
        let content =
            serde_json::to_string_pretty(registry).map_err(|e| format!("序列化注册表失败: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, content).map_err(|e| format!("写入注册表临时文件失败: {e}"))?;
        fs::rename(&tmp, &path).map_err(|e| format!("提交注册表失败: {e}"))
    }

    /// 向 user-added 注册表插入或更新条目（同名则合并，与 Kiro `addPowerToUserAddedRegistry` 一致）
    fn upsert_user_added_entry(entry: UserAddedPowerEntry) -> Result<(), String> {
        let mut registry = Self::load_user_added_registry()?;
        if let Some(existing) = registry.powers.iter_mut().find(|p| p.name == entry.name) {
            *existing = entry;
        } else {
            registry.powers.push(entry);
        }
        Self::save_user_added_registry(&registry)
    }

    /// 从 user-added 注册表移除条目，返回是否确实删除
    fn remove_user_added_entry(name: &str) -> Result<bool, String> {
        let mut registry = Self::load_user_added_registry()?;
        let before = registry.powers.len();
        registry.powers.retain(|p| p.name != name);
        if registry.powers.len() < before {
            Self::save_user_added_registry(&registry)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// 校验目录是否为一个合法 Power（对应 Kiro `isValidPowerDir`）。
    ///
    /// 判据（任一满足即可）：
    /// - 存在 `plugin.json`（新版 agent plugin 形态）
    /// - 存在 `POWER.md`（旧版 legacy 形态）
    pub fn validate_power_dir(dir: &Path) -> Result<(), String> {
        if !dir.is_dir() {
            return Err(format!("路径不是一个目录: {}", dir.display()));
        }
        if dir.join("plugin.json").is_file() || dir.join("POWER.md").is_file() {
            return Ok(());
        }
        Err("所选文件夹不是合法的 Power 目录：需包含 plugin.json 或 POWER.md".to_string())
    }

    /// 由目录名推导 Power 名称（对应 Kiro：basename 转小写，非字母数字与连字符替换为 `-`）
    pub fn derive_power_name(dir: &Path) -> Result<String, String> {
        let base = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let sanitized: String = base
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
            .collect();
        let sanitized = sanitized.trim_matches('-').to_string();
        if sanitized.is_empty() {
            return Err("无法从所选文件夹名推导出合法的 Power 名称".to_string());
        }
        Ok(sanitized)
    }
}

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
            power.installed =
                installed_names.contains(&power.name) || installed_dir_names.contains(&power.name);
        }

        Ok(registry.powers)
    }
}
