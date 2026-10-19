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
// UA 在 Tauri 各平台均为系统默认值：tauri-runtime-wry 只在配置了 userAgent
// 时才调用 with_user_agent（crates/tauri-runtime-wry/src/lib.rs 的
// `if let Some(user_agent) = webview_attributes.user_agent` 分支），
// 否则沿用平台默认 UA，macOS 即 WKWebView 的 "Macintosh; ... Mac OS X"。
//
// 依赖：src-tauri/tauri.conf.json 必须保持不覆盖 userAgent。一旦在
// app.windows[].userAgent 或 tauri.macos.conf.json 等平台覆盖配置里设了自定义
// UA，macOS 识别会静默失效——症状只是侧栏避让消失，不报错，很难排查。
// 改该配置前先确认 UA 仍含 Macintosh。
//
// 探测是同步的，必须在 render 前调用，保证首帧就带对标记。

export type OsName = 'macos' | 'windows' | 'linux' | 'unknown'

// 按 UA 判定平台。顺序有意义：macOS UA 形如
// "Macintosh; Intel Mac OS X 10_15_7"，Windows 形如 "Windows NT 10.0; Win64; x64"，
// Linux 形如 "X11; Linux x86_64"，三者互不重叠，但 macOS 必须优先判，
// 因为桌面模式下的 iPad UA 也含 Macintosh（本项目不面向移动端，无影响）。
export function detectOsFromUa(ua: string): OsName {
  if (!ua) return 'unknown'
  if (/Macintosh|Mac OS X/i.test(ua)) return 'macos'
  if (/Windows/i.test(ua)) return 'windows'
  if (/Linux|X11/i.test(ua)) return 'linux'
  return 'unknown'
}

// 把平台写到 documentElement.dataset.os。CSS 侧只认 [data-os='macos']，
// 探测失败时不会误留白：unknown 与 windows/linux 同等对待。
export function applyOsAttribute(doc: Document = document): OsName {
  const ua = doc.defaultView?.navigator?.userAgent ?? ''
  const os = detectOsFromUa(ua)
  doc.documentElement.dataset.os = os
  return os
}
