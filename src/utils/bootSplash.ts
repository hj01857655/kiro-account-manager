export function dismissBootSplash(doc = document) {
  const splash = doc?.getElementById?.('boot-splash')
  if (!splash) return false

  splash.dataset.state = 'hidden'
  splash.remove()
  return true
}

/** 撤卡兜底阈值：React 因打包损坏 / 运行时异常始终没挂载时，dismissBootSplash 不会被调用，
 *  用户会被永久困在启动卡后面。给得比后端 3 秒兜底宽，开发模式慢加载不会被提前掀开。 */
export const BOOT_SPLASH_FALLBACK_MS = 8000

/**
 * 启动撤卡兜底定时器。
 *
 * 必须在 render 之前、且在打包产物里调用：写在 index.html 的内联 script 会被生产 CSP
 * （`script-src: ["'self"]`，见 tauri.conf.json）直接拦掉，等于没有兜底。
 * 放在模块顶层而非 useEffect 内，是为了在 React 渲染抛错时也能执行到。
 */
export function scheduleBootSplashFallback(
  doc = document,
  timeoutMs: number = BOOT_SPLASH_FALLBACK_MS
) {
  if (typeof doc?.getElementById !== 'function') return
  if (typeof setTimeout !== 'function') return
  setTimeout(() => {
    dismissBootSplash(doc)
  }, timeoutMs)
}
