import test from 'node:test'
import assert from 'node:assert/strict'
import { dismissBootSplash, scheduleBootSplashFallback } from './bootSplash.js'

function makeDoc() {
  const splash = {
    dataset: {} as Record<string, string>,
    remove() {
      removed = true
    }}
  let removed = false
  const documentMock = {
    getElementById(id: string) {
      return id === 'boot-splash' ? splash : null
    }}
  return { documentMock, splash, isRemoved: () => removed }
}

test('dismissBootSplash marks splash hidden and removes it', () => {
  let removed = false
  const splash = {
    dataset: {} as Record<string, string>,
    remove() {
      removed = true
    }}
  const documentMock = {
    getElementById(id: string) {
      return id === 'boot-splash' ? splash : null
    }}

  const result = dismissBootSplash(documentMock as unknown as Document)

  assert.equal(result, true)
  assert.equal(splash.dataset.state, 'hidden')
  assert.equal(removed, true)
})

test('dismissBootSplash returns false when splash is absent', () => {
  const documentMock = {
    getElementById() {
      return null
    }}

  const result = dismissBootSplash(documentMock as unknown as Document)

  assert.equal(result, false)
})

// 兜底定时器必须由打包产物注册：index.html 的内联 script 会被生产 CSP
// （script-src: ['self']）拦掉，写在那里等于没有兜底。
test('scheduleBootSplashFallback removes splash after the timeout', () => {
  const { documentMock, isRemoved } = makeDoc()
  let fired: (() => void) | null = null
  // scheduleBootSplashFallback 读全局 setTimeout（见 bootSplash.ts），mock 必须挂到
  // globalThis 才能被它看到；同步安装/同步恢复，不会泄漏到其它测试
  const originalSetTimeout = globalThis.setTimeout
  globalThis.setTimeout = ((fn: () => void) => {
    fired = fn
    return 0
  }) as unknown as typeof setTimeout

  try {
    scheduleBootSplashFallback(documentMock as unknown as Document, 8000)

    assert.equal(isRemoved(), false, '注册时不能立刻撤，否则盖不住加载期')
    assert.ok(fired, '必须注册一个定时器')
    ;(fired as unknown as () => void)()
    assert.equal(isRemoved(), true)
  } finally {
    globalThis.setTimeout = originalSetTimeout
  }
})

test('scheduleBootSplashFallback tolerates missing document', () => {
  assert.doesNotThrow(() => scheduleBootSplashFallback(null as unknown as Document))
})