// 便携版支持模块
// 检测 exe 同目录下是否存在 .portable 文件，如果存在则使用便携模式
// 便携模式下，仅本应用自己的数据存储在 exe 同目录的 data 子目录中
// Kiro IDE 相关的配置（MCP、Powers、Steering、AWS SSO）保持系统路径不变

use std::path::PathBuf;
use std::sync::OnceLock;

/// 缓存便携模式状态和数据目录路径
static PORTABLE_STATE: OnceLock<PortableState> = OnceLock::new();

struct PortableState {
    is_portable: bool,
    data_dir: PathBuf,
    exe_dir: PathBuf,
}

impl PortableState {
    fn init() -> Self {
        // current_exe() 在 Windows/macOS 桌面应用中几乎不会失败
        // 回退到 "." 是防御性编程，但会打印警告
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| {
                eprintln!("[Portable] 警告: 无法获取可执行文件路径，回退到当前目录");
                PathBuf::from(".")
            });
        
        let portable_marker = exe_dir.join(".portable");
        let is_portable = portable_marker.exists();
        
        let data_dir = if is_portable {
            exe_dir.join("data")
        } else {
            get_default_data_dir()
        };
        
        Self { is_portable, data_dir, exe_dir }
    }
}

/// 获取默认数据目录（非便携模式）
fn get_default_data_dir() -> PathBuf {
    // 优先使用 dirs::data_dir()，失败时尝试环境变量
    // 如果都失败则 panic，因为静默回退到 "." 可能导致数据丢失
    dirs::data_dir().unwrap_or_else(|| {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .expect("无法确定用户数据目录：dirs::data_dir() 失败且 USERPROFILE/HOME 环境变量均未设置");
        PathBuf::from(home)
    })
}

/// 获取便携状态（懒加载并缓存）
fn get_state() -> &'static PortableState {
    PORTABLE_STATE.get_or_init(PortableState::init)
}

/// 检查是否为便携模式
pub fn is_portable() -> bool {
    get_state().is_portable
}

/// 获取数据目录路径
/// 便携模式: exe目录/data
/// 普通模式: %APPDATA% 或 ~/Library/Application Support
pub fn get_data_dir() -> PathBuf {
    get_state().data_dir.clone()
}

/// 获取 exe 所在目录
pub fn get_exe_dir() -> PathBuf {
    get_state().exe_dir.clone()
}

/// 获取应用数据目录 (.kiro-account-manager)
/// 这是唯一需要便携化的目录，存储本应用的账号和设置
pub fn get_app_data_dir() -> PathBuf {
    get_data_dir().join(".kiro-account-manager")
}

/// 首次启动便携模式时，尝试从普通模式导入配置
pub fn try_import_from_normal_mode() -> Result<bool, String> {
    if !is_portable() {
        return Ok(false);
    }
    
    let portable_data_dir = get_app_data_dir();
    
    // 如果便携数据目录已存在且有内容，跳过导入
    if portable_data_dir.exists() {
        match std::fs::read_dir(&portable_data_dir) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    return Ok(false);
                }
            }
            Err(e) => {
                // 无法读取目录(权限等问题)，返回错误而不是继续导入
                return Err(format!("无法读取便携数据目录 ({}): {}", portable_data_dir.display(), e));
            }
        }
    }
    
    // 获取普通模式的数据目录
    let normal_data_dir = get_default_data_dir()
        .join(".kiro-account-manager");
    
    if !normal_data_dir.exists() {
        return Ok(false);
    }
    
    // 创建便携数据目录
    std::fs::create_dir_all(&portable_data_dir)
        .map_err(|e| format!("创建便携数据目录失败 ({}): {}", portable_data_dir.display(), e))?;
    
    // 复制配置文件
    // 注意：这里不使用原子事务（临时目录+重命名），原因：
    // 1. 只复制两个小配置文件，部分失败概率极低
    // 2. 即使失败，用户可手动删除 data 目录重试
    // 3. 添加复杂事务机制会增加代码复杂度，收益不大
    let files_to_copy = ["accounts.json", "app-settings.json"];
    let mut copied = false;
    
    for file in &files_to_copy {
        let src = normal_data_dir.join(file);
        let dst = portable_data_dir.join(file);
        if src.exists() {
            std::fs::copy(&src, &dst)
                .map_err(|e| format!("复制 {} 失败 ({} -> {}): {}", file, src.display(), dst.display(), e))?;
            copied = true;
        }
    }
    
    Ok(copied)
}

/// 确保数据目录存在
pub fn ensure_data_dir() -> Result<(), String> {
    let dir = get_app_data_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建数据目录失败 ({}): {}", dir.display(), e))
}
