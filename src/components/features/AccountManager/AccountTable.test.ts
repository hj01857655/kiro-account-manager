import test from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

test('AccountTable estimates rows by density and corrects with measureElement', async () => {
  const source = await readFile(new URL('./AccountTable.tsx', import.meta.url), 'utf8')

  // 密度与 UI 缩放决定估算行高（comfortable 基准 320），实际高度由 measureElement 校正
  assert.match(source, /density === 'compact' \? 296 : density === 'spacious' \? 344 : 320/)
  assert.match(source, /estimateSize:\s*\(\)\s*=>\s*rowHeight/)
  assert.match(source, /ref=\{rowVirtualizer\.measureElement\}/)
})
