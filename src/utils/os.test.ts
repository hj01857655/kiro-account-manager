import test from 'node:test'
import assert from 'node:assert/strict'
import { applyOsAttribute, detectOsFromUa } from './os.js'

// 三端 WebView 的 UA 均为系统默认值（tauri.conf.json 未覆盖 userAgent）。
const UA = {
  macos: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
  windows: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0',
  linux: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36',
}

test('detectOsFromUa identifies each platform', () => {
  assert.equal(detectOsFromUa(UA.macos), 'macos')
  assert.equal(detectOsFromUa(UA.windows), 'windows')
  assert.equal(detectOsFromUa(UA.linux), 'linux')
})

// macOS 必须优先于 Linux 判定：桌面 UA 不冲突，但 macOS 判在后面会让
// 任何含 "X11" 的 UA 漏判，顺序是防御性的。
test('detectOsFromUa prefers macos over other tokens', () => {
  assert.equal(detectOsFromUa('X11; Linux x86_64 Macintosh'), 'macos')
})

test('detectOsFromUa returns unknown for empty or unrecognized UA', () => {
  assert.equal(detectOsFromUa(''), 'unknown')
  assert.equal(detectOsFromUa('Mozilla/5.0 (compatible; Googlebot/2.1)'), 'unknown')
})

function makeDoc(ua: string) {
  const dataset: Record<string, string> = {}
  return {
    dataset,
    documentElement: { dataset },
    defaultView: { navigator: { userAgent: ua } },
  }
}

test('applyOsAttribute writes data-os onto the document element', () => {
  const doc = makeDoc(UA.macos)
  assert.equal(applyOsAttribute(doc as unknown as Document), 'macos')
  assert.equal(doc.dataset.os, 'macos')

  const win = makeDoc(UA.windows)
  applyOsAttribute(win as unknown as Document)
  assert.equal(win.dataset.os, 'windows')
})

// CSS 侧只有 [data-os='macos'] 留白，所以探测失败必须是「不留白」：
// unknown 与 windows/linux 同等对待，否则空 UA 会让所有平台都出现 36px 留白。
test('applyOsAttribute never marks macos when UA is missing', () => {
  const doc = makeDoc('')
  assert.equal(applyOsAttribute(doc as unknown as Document), 'unknown')
  assert.notEqual(doc.dataset.os, 'macos')
})

test('applyOsAttribute tolerates a document without a browsing context', () => {
  const dataset: Record<string, string> = {}
  const doc = { documentElement: { dataset } }
  assert.doesNotThrow(() => applyOsAttribute(doc as unknown as Document))
  assert.equal(dataset.os, 'unknown')
})
