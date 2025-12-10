/**
 * 统一的 CodeWhisperer 请求头生成
 */

export function createCodeWhispererHeaders(accessToken: string): HeadersInit {
  return {
    "Content-Type": "application/json",
    "Authorization": `Bearer ${accessToken}`,
    "x-amzn-kiro-agent-mode": "spec",
    "x-amz-user-agent": "aws-sdk-js/1.0.18 KiroIDE-0.2.13-66c23a8c5d15afabec89ef9954ef52a119f10d369df04d548fc6c1eac694b0d1",
    "user-agent": "aws-sdk-js/1.0.18 ua/2.1 os/darwin#25.0.0 lang/js md/nodejs#20.16.0 api/codewhispererstreaming#1.0.18 m/E KiroIDE-0.2.13-66c23a8c5d15afabec89ef9954ef52a119f10d369df04d548fc6c1eac694b0d1",
  };
}
