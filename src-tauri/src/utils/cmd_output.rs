// 命令输出解码工具：解决 Windows 中文环境下 GBK/UTF-8 混用导致的乱码

#[cfg(target_os = "windows")]
use encoding_rs::GBK;

/// 将命令输出字节解码为字符串。
/// - Windows: 优先按 UTF-8，失败后按 GBK 解码
/// - 其他平台: UTF-8 lossy
pub fn decode_cmd_output(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        if bytes.is_empty() {
            return String::new();
        }

        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.to_string();
        }

        let (cow, _, had_errors) = GBK.decode(bytes);
        if !had_errors {
            return cow.into_owned();
        }

        String::from_utf8_lossy(bytes).into_owned()
    }

    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}
