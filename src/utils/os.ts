// 平台探测：给 <html> 打 data-os 标记，供 CSS 做三端适配。
//
// 为什么不用 CSS 探测：Tauri 三端 WebView 是 macOS=WKWebView、
// Linux=WebKitGTK（同为 WebKit 系）、Windows=WebView2（Chromium）。
// CSS 层没有任何特性能把 macOS 和 Linux 分开——
//   - @media (-webkit-touch-callout: none) 不是合法媒体特性：
//     -webkit-touch-callout 是元素属性而非媒体特性，按规范整个查询被
//     替换为 not all，恒为假（含取反形式），这是它此前从未生效的原因；
//   - @supports (-webkit-touch-callout: none) 在 WKWebView 与 WebKitGTK
//     上都命中，会把没有交通灯的 Linux 也当成 macOS。
//
// 平台值取自官方插件 tauri-plugin-os 的 platform()，即 Rust 侧
// std::env::consts::OS——编译期常量，不嗅探 UA 字符串，也不依赖任何
// 平台特征巧合。它是同步的：值经 js_init_script 在页面脚本之前注入到
// window.__TAURI_OS_PLUGIN_INTERNALS__，所以 render 前调用即可拿到，
// 首帧不会出现无避让的布局。
//
// 浏览器里跑 dev server（无 Tauri 运行时）时 platform() 会抛错，
// 此时退回 UA 判定，仅用于本地开发；生产打包后一律走插件路径。
//
// 探测失败（含 UA 也认不出）时写 unknown：CSS 侧只有 [data-os='macos']
// 留白，unknown 与 windows/linux 同等对待，不会误留白。

import { platform as tauriPlatform } from '@tauri-apps/plugin-os'

export type OsName = 'macos' | 'windows' | 'linux' | 'unknown'

// plugin-os 的 Platform 取值见 node_modules/@tauri-apps/plugin-os：
// 'linux' | 'macos' | 'ios' | 'freebsd' | ... | 'windows'。
// 只映射本项目部署的三端，其余一律 unknown——CSS 对 unknown 的处理
// 与 linux 相同（无避让 + 细滚动条），对未支持的平台是安全的默认。
function fromTauriPlatform(p: string): OsName {
  switch (p) {
    case 'macos':
      return 'macos'
    case 'windows':
      return 'windows'
    case 'linux':
      return 'linux'
    default:
      return 'unknown'
  }
}

// 浏览器开发兜底：仅当 platform() 不可用（非 Tauri 环境）时才走这里。
// 顺序有意义：macOS UA 形如 "Macintosh; Intel Mac OS X 10_15_7"，
// Windows 形如 "Windows NT 10.0; Win64; x64"，
// Linux 形如 "X11; Linux x86_64"，三者互不重叠，但 macOS 必须优先判。
export function detectOsFromUa(ua: string): OsName {
  if (!ua) return 'unknown'
  if (/Macintosh|Mac OS X/i.test(ua)) return 'macos'
  if (/Windows/i.test(ua)) return 'windows'
  if (/Linux|X11/i.test(ua)) return 'linux'
  return 'unknown'
}

// 依次尝试官方插件、UA；插件不可用（浏览器开发）或抛错时退回 UA。
function detectOs(doc: Document): OsName {
  try {
    return fromTauriPlatform(tauriPlatform())
  } catch {
    // platform() 读取 window.__TAURI_OS_PLUGIN_INTERNALS__，非 Tauri 环境为 undefined
  }
  return detectOsFromUa(doc.defaultView?.navigator?.userAgent ?? '')
}

// 把平台写到 documentElement.dataset.os。CSS 侧只认 [data-os='macos']，
// 探测失败时不会误留白：unknown 与 windows/linux 同等对待。
export function applyOsAttribute(doc: Document = document): OsName {
  const os = detectOs(doc)
  doc.documentElement.dataset.os = os
  return os
}
