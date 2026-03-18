import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const source = await readFile(new URL('./account_cmd.rs', import.meta.url), 'utf8')

assert.match(source, /pub status: Option<String>/)
assert.match(source, /if let Some\(status\) = params\.status \{\s*store\.accounts\[idx\]\.status = status;/s)

console.log('account_cmd update_account status contract looks correct')
