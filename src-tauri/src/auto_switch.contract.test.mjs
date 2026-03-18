import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const autoSwitchSource = await readFile(new URL('./auto_switch.rs', import.meta.url), 'utf8')
const accountSource = await readFile(new URL('./account.rs', import.meta.url), 'utf8')

assert.match(autoSwitchSource, /"banned"\s*\|\s*"封禁"\s*\|\s*"已封禁"\s*\|\s*"已过期"/)
assert.match(autoSwitchSource, /get\("usageBreakdownList"\)/)
assert.match(autoSwitchSource, /get\("currentUsage"\)/)
assert.match(autoSwitchSource, /get\("usageLimit"\)/)

assert.match(accountSource, /"banned"\s*\|\s*"封禁"\s*\|\s*"已封禁"\s*\|\s*"已过期"/)

console.log('auto_switch status and usage contract looks correct')
