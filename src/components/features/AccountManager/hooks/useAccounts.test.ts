import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const source = await readFile(new URL('./useAccounts.ts', import.meta.url), 'utf8')

assert.doesNotMatch(source, /updates:\s*\{\s*status:\s*'banned'\s*\}/)
// invoke 已收拢进 src/api/ 封装层：封禁/失效写状态走 updateAccount({ id, status })
assert.match(source, /await updateAccount\(\{ id, status: 'banned' \}\)/)

console.log('useAccounts banned status wiring looks correct')
