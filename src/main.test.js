import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const source = await readFile(new URL('./main.jsx', import.meta.url), 'utf8')

assert.match(source, /const hasCurrentTauriWindow = \(\) => Boolean\(window\.__TAURI_INTERNALS__\?\.metadata\?\.currentWindow\)/)
assert.match(source, /if \(!hasCurrentTauriWindow\(\)\) return/)
assert.match(source, /getCurrentWindow\(\)\.show\(\)\.catch\?\.\(\(\) => \{\}\)/)

console.log('main window bootstrap is guarded')
