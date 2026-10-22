import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const source = await readFile(new URL('./EditAccountModal.tsx', import.meta.url), 'utf8')

// invoke 已收拢进 src/api/ 封装层：保存走 updateAccount(params)，params 仍整体传参
assert.match(source, /await updateAccount<Account>\(params\)/)
assert.doesNotMatch(source, /invoke\('update_account'/)

console.log('EditAccountModal update_account wiring looks correct')
