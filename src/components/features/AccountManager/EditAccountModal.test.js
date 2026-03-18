import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const source = await readFile(new URL('./EditAccountModal.jsx', import.meta.url), 'utf8')

assert.match(source, /await invoke\('update_account',\s*params\s*\)/)
assert.doesNotMatch(source, /await invoke\('update_account',\s*\{\s*params\s*\}\s*\)/)

console.log('EditAccountModal update_account wiring looks correct')
