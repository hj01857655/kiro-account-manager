import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const mainSource = await readFile(new URL('./main.tsx', import.meta.url), 'utf8')
const appSource = await readFile(new URL('./App.tsx', import.meta.url), 'utf8')

assert.doesNotMatch(mainSource, /getCurrentWindow\(\)\.show\(\)/)
assert.doesNotMatch(mainSource, /window\.__TAURI_INTERNALS__\?\.metadata\?\.currentWindow/)
assert.doesNotMatch(mainSource, /invoke\('show_main_window'\)/)
assert.match(appSource, /dismissBootSplash\(\)/)
// invoke 已收拢进 src/api/ 封装层：App.tsx 只调 showMainWindow()，不直接写 invoke
assert.match(appSource, /showMainWindow\(\)/)

console.log('main window reveal is deferred to App first paint stage')
