import test from 'node:test'
import assert from 'node:assert/strict'
import { squarify } from './ContextMap.js'

// 面积守恒：所有瓦片的面积之和 = 铺入区域的面积
test('squarify preserves total area', () => {
  const vals = [
    { v: 50, kind: 'system' },
    { v: 30, kind: 'tools' },
    { v: 15, kind: 'chat' },
    { v: 5, kind: 'files' },
  ]
  const tiles = squarify(vals, 0, 0, 100, 100)
  const area = tiles.reduce((s, t) => s + t.w * t.h, 0)
  assert.ok(Math.abs(area - 100 * 100) < 1e-6, `面积应守恒，实际 ${area}`)
})

// 面积按权重分配：每块的面积比例 = 它的 tokens 比例
test('squarify allocates area proportionally to weight', () => {
  const vals = [
    { v: 60, kind: 'a' },
    { v: 30, kind: 'b' },
    { v: 10, kind: 'c' },
  ]
  const tiles = squarify(vals, 0, 0, 100, 100)
  const total = vals.reduce((s, v) => s + v.v, 0)
  for (const t of tiles) {
    const want = (t.tokens / total) * 10000
    assert.ok(
      Math.abs(t.w * t.h - want) < 1,
      `${t.kind} 面积 ${t.w * t.h} 应接近 ${want}`,
    )
  }
})

// 不重叠：任意两块瓦片的矩形不交叠
test('squarify tiles do not overlap', () => {
  const vals = [
    { v: 40, kind: 'a' },
    { v: 25, kind: 'b' },
    { v: 20, kind: 'c' },
    { v: 10, kind: 'd' },
    { v: 5, kind: 'e' },
  ]
  const tiles = squarify(vals, 0, 0, 100, 100)
  for (let i = 0; i < tiles.length; i++) {
    for (let j = i + 1; j < tiles.length; j++) {
      const a = tiles[i]
      const b = tiles[j]
      const overlapX = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x)
      const overlapY = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y)
      const overlap = overlapX > 1e-6 && overlapY > 1e-6
      assert.ok(!overlap, `${a.kind} 与 ${b.kind} 重叠`)
    }
  }
})

// 坐标合法：全部落在给定区域内
test('squarify keeps tiles inside the given rect', () => {
  const tiles = squarify(
    [{ v: 10, kind: 'a' }, { v: 20, kind: 'b' }, { v: 70, kind: 'c' }],
    10,
    20,
    80,
    60,
  )
  for (const t of tiles) {
    assert.ok(t.x >= 10 - 1e-6, `${t.kind} x 越界: ${t.x}`)
    assert.ok(t.y >= 20 - 1e-6, `${t.kind} y 越界: ${t.y}`)
    assert.ok(t.x + t.w <= 10 + 80 + 1e-6, `${t.kind} 右越界`)
    assert.ok(t.y + t.h <= 20 + 60 + 1e-6, `${t.kind} 下越界`)
    assert.ok(t.w > 0 && t.h > 0, `${t.kind} 尺寸非正`)
  }
})

// 单一类别：铺满整块区域
test('squarify fills the rect for a single kind', () => {
  const tiles = squarify([{ v: 42, kind: 'only' }], 0, 0, 100, 100)
  assert.equal(tiles.length, 1)
  assert.ok(Math.abs(tiles[0].w - 100) < 1e-6)
  assert.ok(Math.abs(tiles[0].h - 100) < 1e-6)
})

// 零权重与非法输入：不崩、不产生瓦片
test('squarify ignores zero and invalid input', () => {
  assert.deepEqual(squarify([], 0, 0, 100, 100), [])
  assert.deepEqual(squarify([{ v: 0, kind: 'x' }], 0, 0, 100, 100), [])
  assert.deepEqual(squarify([{ v: 10, kind: 'x' }], 0, 0, 0, 100), [])
})

// 比例悬殊时大块仍占大头（不被小块的保底挤掉）
test('squarify keeps the dominant kind dominant', () => {
  const tiles = squarify(
    [{ v: 1000, kind: 'big' }, { v: 1, kind: 'tiny' }],
    0,
    0,
    100,
    100,
  )
  const big = tiles.find((t) => t.kind === 'big')!
  const tiny = tiles.find((t) => t.kind === 'tiny')!
  assert.ok(big.w * big.h > tiny.w * tiny.h * 50, '大块应远大于小块')
})
