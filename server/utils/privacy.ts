/**
 * Privacy Utility Functions
 * 
 * 提供隐私保护相关的工具函数，用于脱敏处理敏感信息
 */

/**
 * 创建 token 预览（***+最后10个字符）
 * 
 * @param token 原始 token 字符串
 * @returns 脱敏后的 token 预览
 * 
 * @example
 * createTokenPreview("1234567890abcdefghij") // "***0abcdefghij"
 * createTokenPreview("short") // "*****"
 */
export function createTokenPreview(token: string): string {
  if (token.length <= 10) {
    return "*".repeat(token.length);
  }
  const suffix = token.substring(token.length - 10);
  return "***" + suffix;
}

/**
 * 脱敏邮箱地址
 * 
 * 保留用户名的前2位和后2位，保留域名的次级域名和顶级域名
 * 
 * @param email 原始邮箱地址
 * @returns 脱敏后的邮箱地址
 * 
 * @example
 * maskEmail("john.doe@gmail.com") // "jo****oe@*****.com"
 * maskEmail("user@sun.edu.pl") // "us**@***.edu.pl"
 * maskEmail("未知用户") // "未知用户" (保持不变)
 */
export function maskEmail(email: string): string {
  // 特殊值不处理
  if (!email || email === "未知用户" || email === "未获取") {
    return email;
  }

  // 分割邮箱为用户名和域名
  const parts = email.split("@");
  if (parts.length !== 2) {
    // 不是有效的邮箱格式，返回原值
    return email;
  }

  const username = parts[0];
  const domain = parts[1];

  // 脱敏用户名：保留前2位和后2位
  let maskedUsername: string;
  if (username.length <= 4) {
    maskedUsername = "*".repeat(username.length);
  } else {
    const prefix = username.substring(0, 2);
    const suffix = username.substring(username.length - 2);
    const middleLen = username.length - 4;
    maskedUsername = prefix + "*".repeat(middleLen) + suffix;
  }

  // 脱敏域名：保留 TLD 和次级域名
  const domainParts = domain.split(".");
  let maskedDomain: string;

  if (domainParts.length === 1) {
    // 单级域名，全部脱敏
    maskedDomain = "*".repeat(domain.length);
  } else if (domainParts.length === 2) {
    // 二级域名，如 gmail.com -> *****.com
    maskedDomain = "*".repeat(domainParts[0].length) + "." + domainParts[1];
  } else {
    // 多级域名，如 sun.edu.pl -> ***.edu.pl
    const maskedParts: string[] = [];
    
    // 脱敏除最后两级外的所有级别
    for (let i = 0; i < domainParts.length - 2; i++) {
      maskedParts.push("*".repeat(domainParts[i].length));
    }
    
    // 保留最后两级
    maskedParts.push(domainParts[domainParts.length - 2]);
    maskedParts.push(domainParts[domainParts.length - 1]);
    
    maskedDomain = maskedParts.join(".");
  }

  return maskedUsername + "@" + maskedDomain;
}

/**
 * 脱敏客户端 ID
 * 
 * 保留前5位和后3位
 * 
 * @param clientId 原始客户端 ID
 * @returns 脱敏后的客户端 ID
 * 
 * @example
 * maskClientId("1234567890abcd") // "12345***bcd"
 * maskClientId("short") // "short" (长度不足不处理)
 */
export function maskClientId(clientId: string): string {
  if (clientId.length <= 10) {
    return clientId;
  }
  
  const prefix = clientId.substring(0, 5);
  const suffix = clientId.substring(clientId.length - 3);
  return prefix + "***" + suffix;
}

/**
 * 通用脱敏函数
 * 
 * 保留指定数量的前缀和后缀字符
 * 
 * @param text 原始文本
 * @param prefixLen 保留的前缀长度
 * @param suffixLen 保留的后缀长度
 * @param maskChar 脱敏字符，默认为 "*"
 * @returns 脱敏后的文本
 * 
 * @example
 * maskText("sensitive-data", 3, 2) // "sen***ta"
 */
export function maskText(
  text: string,
  prefixLen: number,
  suffixLen: number,
  maskChar = "*",
): string {
  const totalLen = prefixLen + suffixLen;
  
  if (text.length <= totalLen) {
    return maskChar.repeat(text.length);
  }
  
  const prefix = text.substring(0, prefixLen);
  const suffix = text.substring(text.length - suffixLen);
  const middleLen = text.length - totalLen;
  
  return prefix + maskChar.repeat(middleLen) + suffix;
}
