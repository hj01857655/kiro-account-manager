// Powers 管理（读取/安装/卸载 Powers）

use crate::mcp::McpConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowersRegistry {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub powers: HashMap<String, PowerInfo>,
    #[serde(default, rename = "repoSources")]
    pub repo_sources: HashMap<String, RepoSource>,
    #[serde(default, rename = "kiroRecommendedRepo")]
    pub kiro_recommended_repo: Option<KiroRecommendedRepo>,
    #[serde(default, rename = "lastUpdated")]
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoSource {
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub enabled: bool,
    pub branch: String,
    pub last_commit_sha: Option<String>,
    pub clone_url: String,
    pub path_in_repo: String,
    pub local_path: Option<String>,
    pub cloned_at: Option<String>,
    pub power_count: i32,
    pub powers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KiroRecommendedRepo {
    pub url: String,
    pub last_fetch: Option<String>,
    pub power_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub repository_url: Option<String>,
    #[serde(default)]
    pub repository_clone_url: Option<String>,
    #[serde(default)]
    pub repository_branch: Option<String>,
    #[serde(default)]
    pub path_in_repo: Option<String>,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub install_path: Option<String>,
    #[serde(default)]
    pub installed_commit_sha: Option<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,
    #[serde(default)]
    pub source: Option<PowerSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub clone_id: Option<String>,
}

impl PowersRegistry {
    /// 获取 Powers 目录路径（始终使用系统路径，因为 Kiro IDE 从这里读取）
    pub fn powers_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("powers"))
    }

    /// 获取 Powers 注册表文件路径
    pub fn registry_path() -> Option<PathBuf> {
        Self::powers_dir().map(|p| p.join("registry.json"))
    }

    /// 读取注册表
    pub fn load() -> Result<Self, String> {
        let path = Self::registry_path().ok_or("无法获取用户目录")?;
        
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取注册表失败: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("解析注册表失败: {}", e))
    }

    /// 保存注册表
    pub fn save(&self) -> Result<(), String> {
        let path = Self::registry_path().ok_or("无法获取用户目录")?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化失败: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("写入失败: {}", e))
    }

    /// 获取已安装的 Powers 列表
    pub fn get_installed(&self) -> Vec<PowerInfo> {
        self.powers
            .values()
            .filter(|p| p.installed)
            .cloned()
            .collect()
    }

    /// 获取所有 Powers 列表（包括未安装的）
    pub fn get_all(&self) -> Vec<PowerInfo> {
        self.powers.values().cloned().collect()
    }

    /// 安装 Power
    pub fn install_power(&mut self, name: &str) -> Result<PowerInfo, String> {
        let power = self.powers.get(name)
            .ok_or_else(|| format!("Power '{}' 不存在", name))?
            .clone();
        
        if power.installed {
            return Err(format!("Power '{}' 已安装", name));
        }

        let powers_dir = Self::powers_dir().ok_or("无法获取 Powers 目录")?;
        let repos_dir = powers_dir.join("repos");
        let installed_dir = powers_dir.join("installed");
        
        // 获取仓库信息
        let clone_url = power.repository_clone_url.as_ref()
            .ok_or("缺少仓库 clone URL")?;
        let branch = power.repository_branch.as_deref().unwrap_or("main");
        let path_in_repo = power.path_in_repo.as_deref().unwrap_or(&power.name);
        
        // 仓库本地路径
        let repo_local_dir = repos_dir.join(name);
        
        // 如果仓库不存在，先 clone
        if !repo_local_dir.exists() {
            fs::create_dir_all(&repos_dir).ok();
            
            let output = Command::new("git")
                .args(["clone", "--depth", "1", "--branch", branch, clone_url])
                .arg(&repo_local_dir)
                .output()
                .map_err(|e| format!("执行 git clone 失败: {}", e))?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("git clone 失败: {}", stderr));
            }
        }
        
        // 复制 power 文件到 installed 目录
        let source_dir = repo_local_dir.join(path_in_repo);
        let target_dir = installed_dir.join(name);
        
        if !source_dir.exists() {
            return Err(format!("源目录不存在: {:?}", source_dir));
        }
        
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("创建目录失败: {}", e))?;
        
        // 复制文件
        copy_dir_contents(&source_dir, &target_dir)?;
        
        // 获取 commit sha
        let commit_sha = get_git_commit_sha(&repo_local_dir);
        
        // 更新注册表
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(p) = self.powers.get_mut(name) {
            p.installed = true;
            p.installed_at = Some(now.clone());
            p.install_path = Some(target_dir.to_string_lossy().to_string());
            p.installed_commit_sha = commit_sha;
            p.source = Some(PowerSource {
                source_type: "registry".to_string(),
                clone_id: Some(format!("clone-{}", name)),
            });
        }
        
        // 更新 repoSources
        self.repo_sources.insert(format!("clone-{}", name), RepoSource {
            name: name.to_string(),
            source_type: "git".to_string(),
            enabled: true,
            branch: branch.to_string(),
            last_commit_sha: self.powers.get(name).and_then(|p| p.installed_commit_sha.clone()),
            clone_url: clone_url.to_string(),
            path_in_repo: path_in_repo.to_string(),
            local_path: Some(repo_local_dir.to_string_lossy().to_string()),
            cloned_at: Some(now),
            power_count: 1,
            powers: vec![name.to_string()],
        });
        
        self.last_updated = Some(chrono::Utc::now().to_rfc3339());
        self.save()?;
        
        // 同步 MCP 配置
        let mcp_json_path = target_dir.join("mcp.json");
        if mcp_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&mcp_json_path) {
                if let Ok(mcp_config) = serde_json::from_str(&content) {
                    McpConfig::add_power_mcp(name, mcp_config).ok();
                }
            }
        }
        
        self.powers.get(name).cloned().ok_or("更新失败".to_string())
    }

    /// 卸载 Power
    pub fn uninstall_power(&mut self, name: &str) -> Result<(), String> {
        let power = self.powers.get(name)
            .ok_or_else(|| format!("Power '{}' 不存在", name))?;
        
        if !power.installed {
            return Err(format!("Power '{}' 未安装", name));
        }

        let powers_dir = Self::powers_dir().ok_or("无法获取 Powers 目录")?;
        let installed_dir = powers_dir.join("installed").join(name);
        
        // 删除已安装目录
        if installed_dir.exists() {
            fs::remove_dir_all(&installed_dir)
                .map_err(|e| format!("删除目录失败: {}", e))?;
        }
        
        // 移除 MCP 配置
        McpConfig::remove_power_mcp(name).ok();
        
        // 更新注册表
        if let Some(p) = self.powers.get_mut(name) {
            p.installed = false;
            p.installed_at = None;
            p.install_path = None;
            // 保留 installed_commit_sha 以便重新安装时使用
        }
        
        self.last_updated = Some(chrono::Utc::now().to_rfc3339());
        self.save()?;
        
        Ok(())
    }
}

/// 复制目录内容
fn copy_dir_contents(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name().ok_or("无效文件名")?;
        let dst_path = dst.join(file_name);
        
        if path.is_dir() {
            // 跳过 .git 目录
            if file_name == ".git" {
                continue;
            }
            fs::create_dir_all(&dst_path).ok();
            copy_dir_contents(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)
                .map_err(|e| format!("复制文件失败: {}", e))?;
        }
    }
    Ok(())
}

/// 获取 git commit sha
fn get_git_commit_sha(repo_dir: &PathBuf) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_dir)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}
