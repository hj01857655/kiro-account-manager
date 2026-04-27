// Deep Link 协议注册管理
// 确保 kiro:// 协议始终指向当前运行的应用

/// 确保 kiro:// 协议指向当前运行的应用
/// 
/// 在 Windows 上更新注册表，确保协议处理程序指向当前 exe。
/// 这解决了多版本共存或应用移动后的协议注册问题。
/// 
/// 在其他平台上，协议注册由系统/安装程序处理，无需运行时更新。
#[cfg(windows)]
pub fn ensure_protocol_registration() -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get current exe path: {e}"))?
        .display()
        .to_string();
    
    let command = format!("\"{exe_path}\" \"%1\"");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    for scheme in ["kiro", "kiro-account-manager"] {
        register_protocol_scheme(&hkcu, scheme, &command)?;
    }

    Ok(())
}

#[cfg(windows)]
fn register_protocol_scheme(
    hkcu: &winreg::RegKey,
    scheme: &str,
    command: &str,
) -> Result<(), String> {
    let class_path = format!("Software\\Classes\\{scheme}");
    
    let (class_key, _) = hkcu
        .create_subkey(&class_path)
        .map_err(|e| format!("Failed to create registry key for {scheme}: {e}"))?;
    
    class_key
        .set_value("", &format!("URL:{scheme} Protocol"))
        .map_err(|e| format!("Failed to set protocol description for {scheme}: {e}"))?;
    
    class_key
        .set_value("URL Protocol", &"")
        .map_err(|e| format!("Failed to set URL Protocol flag for {scheme}: {e}"))?;

    let (cmd_key, _) = hkcu
        .create_subkey(format!("{class_path}\\shell\\open\\command"))
        .map_err(|e| format!("Failed to create command key for {scheme}: {e}"))?;
    
    cmd_key
        .set_value("", &command)
        .map_err(|e| format!("Failed to set command for {scheme}: {e}"))?;

    Ok(())
}

#[cfg(not(windows))]
pub fn ensure_protocol_registration() -> Result<(), String> {
    Ok(())
}
