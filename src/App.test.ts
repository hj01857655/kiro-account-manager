import test from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

test('App startup does not gate the whole shell behind auth loading', async () => {
  const source = await readFile(new URL('./App.tsx', import.meta.url), 'utf8')

  assert.match(source, /const url = new URL\(window\.location\.href\)/)
  assert.doesNotMatch(source, /const \[loading, setLoading\] = useState\(true\)/)

  assert.doesNotMatch(source, /setLoading\(false\)/)
  // invoke 已收拢进 src/api/ 封装层：App.tsx 只调 showMainWindow()，不直接写 invoke
  assert.match(source, /import \{ showMainWindow \} from '\.\/api\/systemApi'/)
  assert.match(source, /requestAnimationFrame\(\(\) => \{\s*requestAnimationFrame\(\(\) => \{\s*splashDismissedRef\.current = true\s*dismissBootSplash\(\)\s*showMainWindow\(\)/)
})
