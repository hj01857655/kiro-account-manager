import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const source = await readFile(new URL('./useAccounts.js', import.meta.url), 'utf8')

assert.doesNotMatch(source, /updates:\s*\{\s*status:\s*'banned'\s*\}/)
assert.match(source, /invoke\('update_account',\s*\{\s*id,\s*status:\s*'banned'\s*\}\)/)

console.log('useAccounts banned status wiring looks correct')
