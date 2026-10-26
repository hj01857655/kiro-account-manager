import test from 'node:test'
import assert from 'node:assert/strict'
import { inferWindow, type ContextLogRow } from './ContextPanel.js'

const row = (over: Partial<ContextLogRow> = {}): ContextLogRow => ({
  occurredAt: '2026-10-26 12:00:00',
  statusCode: 200,
  ...over,
})

// 窗口的优先级：后端给的真实窗口 > 百分比反推 > 未知（不猜）
test('inferWindow prefers the backend-provided window', () => {
  const r = row({
    contextUsagePercentage: 10,
    prompt: { tokens: 20_000, counted: false, window: 1_000_000 },
  })
  assert.equal(inferWindow(r, 20_000), 1_000_000, '后端窗口应优先于百分比反推')
})

test('inferWindow falls back to reverse-engineering the percentage', () => {
  const r = row({ contextUsagePercentage: 10 })
  assert.equal(inferWindow(r, 20_000), 200_000, '20k / 10% = 200k')
})

test('inferWindow returns null when nothing is known', () => {
  assert.equal(inferWindow(row(), 20_000), null, '既无窗口也无百分比时为 null')
  assert.equal(inferWindow(row({ contextUsagePercentage: 0 }), 20_000), null, '百分比为 0 不反推')
  assert.equal(
    inferWindow(row({ contextUsagePercentage: 10, prompt: { tokens: 0, counted: false, window: null } }), 0),
    null,
    'tokens 为 0 不反推',
  )
})

// 后端窗口为 0/负数视为无效，回落到百分比反推
test('inferWindow ignores a nonsensical backend window', () => {
  const r = row({
    contextUsagePercentage: 10,
    prompt: { tokens: 20_000, counted: false, window: 0 },
  })
  assert.equal(inferWindow(r, 20_000), 200_000, '无效窗口应回落到百分比反推')
})
