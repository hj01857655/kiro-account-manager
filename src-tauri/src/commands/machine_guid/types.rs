// 机器码相关数据类型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMachineInfo {
    pub machine_guid: Option<String>,
    pub backup_exists: bool,
    pub backup_time: Option<String>,
    pub os_type: String,
    pub can_modify: bool,
    pub requires_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineGuidBackup {
    pub machine_guid: String,
    pub backup_time: String,
    pub computer_name: Option<String>,
    pub os_type: Option<String>,
}
