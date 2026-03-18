import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const source = await readFile(new URL('./useAutoSwitch.js', import.meta.url), 'utf8')

assert.match(source, /invoke\('get_kiro_local_token'\)/)
assert.doesNotMatch(source, /invoke\('get_local_token'\)/)
assert.match(source, /invoke\('switch_kiro_account',\s*\{\s*params\s*\}\)/)
assert.doesNotMatch(source, /invoke\('switch_account'/)
assert.doesNotMatch(source, /updates:\s*\{\s*status:\s*'banned'\s*\}/)
assert.match(source, /invoke\('update_account',\s*\{\s*id:\s*currentAccount\.id,\s*status:\s*'banned'\s*\}\)/)

console.log('useAutoSwitch command wiring looks correct')
