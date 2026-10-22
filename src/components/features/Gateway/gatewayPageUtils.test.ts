import test from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import {
  buildClientSamples,
  buildGatewayBaseUrl,
  buildGatewayClientRecipes,
  buildGatewayConnectHost,
  buildGatewayIntegrationSummary,
  buildGatewaySecuritySummary,
  CLIENT_API_KEY,
  createGatewayFieldErrors,
  formatGatewayAccountOptionLabel,
} from './gatewayPageUtils.js'

/** 测试用翻译 stub：原样返回 key，插值变量以 JSON 追加，便于断言走了哪条文案。 */
const t = (key: string, vars?: Record<string, any>) =>
  vars ? `${key} ${JSON.stringify(vars)}` : key

const loadLocale = async (locale: string): Promise<Record<string, string>> =>
  JSON.parse(await readFile(new URL('../../../../locales/' + locale + '.json', import.meta.url), 'utf-8'))

test('locale keys referenced by gateway page exist in all three languages', async () => {
  for (const locale of ['zh-CN', 'en', 'ru']) {
    const messages = await loadLocale(locale)

    for (const key of [
      'gateway.tabOverview',
      'gateway.tabRequestLogs',
      'gateway.tabSettings',
      'gatewaySummary.security.keysFixed',
      'gatewayLogs.colClientTool',
    ]) {
      assert.equal(typeof messages[key], 'string', locale + ' 缺少 ' + key)
    }
  }
})

test('createGatewayFieldErrors rejects unsupported host values', () => {
  const errors = createGatewayFieldErrors({
    host: 'bad host',
    port: 8765,
    region: 'us-east-1',
    accountMode: 'single',
    accountId: 'account-1',
    localOnly: true,
    allowedIpsText: ''
  }, t)

  assert.equal(errors.host, 'gatewaySummary.field.hostInvalid')
})

test('createGatewayFieldErrors requires allowlist when remote access is enabled', () => {
  const errors = createGatewayFieldErrors({
    host: '0.0.0.0',
    port: 8765,
    region: 'us-east-1',
    accountMode: 'single',
    accountId: 'account-1',
    localOnly: false,
    allowedIpsText: ' \n '
  }, t)

  assert.equal(errors.allowedIpsText, 'gatewaySummary.field.allowlistRequired')
})

test('createGatewayFieldErrors reports no client key error: key is fixed', () => {
  // 密钥固定为 kiro 后不再需要用户配置，空配置也不该报 keyRequired
  const errors = createGatewayFieldErrors({
    host: '127.0.0.1',
    port: 8765,
    region: 'us-east-1',
    accountMode: 'single',
    accountId: 'account-1',
    localOnly: true,
    allowedIpsText: ''
  }, t)

  assert.equal(errors.clientApiKeysText, undefined)
})

test('buildClientSamples exposes the fixed kiro key for both display and copy', () => {
  const samples = buildClientSamples('http://127.0.0.1:8765')

  // 密钥固定且非秘密：展示与复制同一个值
  assert.ok(samples.anthropic.env.includes('ANTHROPIC_API_KEY=' + CLIENT_API_KEY))
  assert.ok(samples.openai.env.includes('OPENAI_API_KEY=' + CLIENT_API_KEY))
  assert.ok(samples.copy.anthropic.includes('ANTHROPIC_API_KEY=' + CLIENT_API_KEY))
  assert.ok(samples.copy.openai.includes('OPENAI_BASE_URL=http://127.0.0.1:8765/v1'))

  const recipes = buildGatewayClientRecipes({
    baseUrl: 'http://127.0.0.1:8765',
    samples,
    t
  })

  assert.equal(recipes.length, 3)
  for (const recipe of recipes) {
    assert.equal(recipe.key, CLIENT_API_KEY)
    assert.equal(recipe.keyMasked, CLIENT_API_KEY)
  }
  assert.equal(recipes[0].baseUrl, 'http://127.0.0.1:8765')
  assert.equal(recipes[1].baseUrl, 'http://127.0.0.1:8765/v1')
  assert.equal(recipes[1].copySample, samples.copy.openai)
  assert.equal(recipes[2].copySample, samples.copy.openai)
})

test('buildGatewaySecuritySummary reports the fixed key state', () => {
  const summary = buildGatewaySecuritySummary({
    config: {
      localOnly: false,
      allowedIpsText: '10.0.0.0/24',
      logLevel: 'info'
    },
    t
  })

  assert.equal(summary.allowedIpsCount, 1)
  assert.equal(summary.apiKeyState, 'gatewaySummary.security.keysFixed')
})

test('buildGatewayIntegrationSummary labels auth with the fixed key', () => {
  const summary = buildGatewayIntegrationSummary({
    baseUrl: 'http://127.0.0.1:8765',
    logDir: 'C:/tmp/logs',
    errorHistory: [],
    t
  })

  assert.equal(summary.authLabel, 'Bearer ' + CLIENT_API_KEY)
})

test('formatGatewayAccountOptionLabel falls back to unknown account', () => {
  const label = formatGatewayAccountOptionLabel({}, t)
  assert.ok(label.includes('gatewaySummary.unknownAccount'))
  assert.ok(label.includes('gatewaySummary.account.remaining'))
})

test('buildGatewayBaseUrl formats plain and bracketed hosts', () => {
  assert.equal(buildGatewayConnectHost('0.0.0.0', false), 'localhost')
  assert.equal(buildGatewayBaseUrl('::1', 8765, true), 'http://[::1]:8765')
  assert.equal(buildGatewayBaseUrl('0.0.0.0', 8765, false), 'http://localhost:8765')
})
