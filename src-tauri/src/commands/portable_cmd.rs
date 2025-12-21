// 便携模式命令

use serde::{Deserialize, Serialize};
use crate::portable;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableInfo {
    pub is_portable: bool,
    pub data_dir: String,
    pub exe_dir: String,
}

#[tauri::command]
pub fn get_portable_info() -> PortableInfo {
    PortableInfo {
        is_portable: portable::is_portable(),
        data_dir: portable::get_data_dir().to_string_lossy().to_string(),
        exe_dir: portable::get_exe_dir().to_string_lossy().to_string(),
    }
}
