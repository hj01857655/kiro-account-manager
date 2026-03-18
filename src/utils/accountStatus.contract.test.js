import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const autoRefresh = await readFile(new URL('../hooks/useAutoRefresh.js', import.meta.url), 'utf8')
const useAccounts = await readFile(new URL('../components/features/AccountManager/hooks/useAccounts.js', import.meta.url), 'utf8')
const importModal = await readFile(new URL('../components/features/AccountManager/ImportAccountModal.jsx', import.meta.url), 'utf8')

assert.match(autoRefresh, /isBannedStatus/)
assert.doesNotMatch(autoRefresh, /status !== 'banned'/)

assert.match(useAccounts, /isBannedStatus/)
assert.doesNotMatch(useAccounts, /status === 'banned'/)
assert.doesNotMatch(useAccounts, /status !== 'banned'/)

assert.match(importModal, /isBannedStatus/)
assert.doesNotMatch(importModal, /status === 'banned'/)

console.log('account status compatibility wiring looks correct')
