// MCP 服务器预设模板

export const MCP_TEMPLATES = {
  'js-reverse': {
    command: 'npx',
    args: ['js-reverse-mcp'],
    env: {},
    disabled: false,
    autoApprove: ['*']
  },
  fetch: {
    command: 'uvx',
    args: ['mcp-server-fetch'],
    env: {},
    disabled: false,
    autoApprove: ['*']
  },
  memory: {
    command: 'npx',
    args: ['-y', '@modelcontextprotocol/server-memory'],
    env: {},
    disabled: false,
    autoApprove: []
  },
  context7: {
    command: 'npx',
    args: ['-y', '@upstash/context7-mcp@latest'],
    env: {},
    disabled: false,
    autoApprove: []
  },
  thinking: {
    command: 'npx',
    args: ['-y', '@modelcontextprotocol/server-sequential-thinking'],
    env: {},
    disabled: false,
    autoApprove: ['*']
  },
  'chrome-devtools': {
    command: 'npx',
    args: [
      '-y',
      'chrome-devtools-mcp@latest',
      '--channel=stable',
      '--headless=false',
      '--isolated=true',
      '--viewport=1920x1080',
      '--chromeArg=--incognito'
    ],
    env: {
      SystemRoot: 'C:\\Windows',
      PROGRAMFILES: 'C:\\Program Files'
    },
    disabled: false,
    autoApprove: ['*']
  },
  codegraph: {
    command: 'codegraph',
    args: ['serve', '--mcp'],
    env: {},
    disabled: false,
    autoApprove: ['*']
  }
}
