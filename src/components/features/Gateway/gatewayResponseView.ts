/**
 * 网关响应体的三种查看方式，以及流式帧解析。
 *
 * 这份逻辑原来只存在于概览的「最近请求」展开区，协议实测页需要完全一样的
 * 能力（同一批响应、同一套 SSE 形状），所以提到共享层：两处展示同一份响应时
 * 不能各算一套，否则「概览里能读出正文、实测页读不出来」这类差异会被当成 bug。
 */

export type ResponseView = 'raw' | 'json' | 'text'

/// 取一行 SSE 的 data 负载；不是 data 行返回 null。
function dataPayload(line: string): string | null {
  const trimmed = line.trim()
  if (!trimmed.startsWith('data:')) return null
  const payload = trimmed.slice(5).trim()
  if (!payload || payload === '[DONE]') return null
  return payload
}

/**
 * 按查看方式渲染响应体。
 *
 * raw  = 后端存的是什么就显示什么；
 * json = 尝试美化，失败则逐帧美化（流式响应整块永远不是合法 JSON，
 *        不拆帧的话这个视图对 stream=true 等于原文）；
 * text = 只抽模型说的正文，三个入口各有一套事件形状。
 */
export function renderResponseBody(body: string, view: ResponseView): string {
  if (!body) return ''
  if (view === 'raw') return body

  if (view === 'json') {
    try {
      return JSON.stringify(JSON.parse(body), null, 2)
    } catch {
      return body
        .split('\n')
        .map((line) => {
          const payload = dataPayload(line)
          if (payload === null) return line
          try {
            return `data: ${JSON.stringify(JSON.parse(payload), null, 2)}`
          } catch {
            return line
          }
        })
        .join('\n')
    }
  }

  // text：OpenAI Chat 是 choices[0].delta.content，Anthropic 是
  // content_block_delta.delta.text，Responses 是 response.output_text.delta.delta。
  // 少认一种，对应协议的流式响应就抽不出正文。
  const chunks: string[] = []
  for (const line of body.split('\n')) {
    const payload = dataPayload(line)
    if (payload === null) continue
    try {
      const parsed = JSON.parse(payload)
      const candidate =
        parsed?.choices?.[0]?.delta?.content ??
        (parsed?.type === 'content_block_delta' ? parsed?.delta?.text : undefined) ??
        (parsed?.type === 'response.output_text.delta' ? parsed?.delta : undefined)
      if (typeof candidate === 'string') chunks.push(candidate)
    } catch {
      // 非 JSON 的 data 行（裸文本流）原样保留
      chunks.push(payload)
    }
  }
  if (chunks.length > 0) return chunks.join('')

  // 不是 SSE，按普通 JSON 取正文：三个入口的响应结构各不相同
  try {
    const parsed = JSON.parse(body)
    const content =
      parsed?.choices?.[0]?.message?.content ??
      parsed?.content?.find?.((block: { type?: string }) => block?.type === 'text')?.text ??
      parsed?.output_text ??
      // Responses 非流式的正文在 output[].content[].text
      (Array.isArray(parsed?.output)
        ? parsed.output
            .flatMap((item: { content?: { type?: string; text?: string }[] }) => item?.content ?? [])
            .filter((block: { type?: string }) => block?.type === 'output_text')
            .map((block: { text?: string }) => block.text ?? '')
            .join('')
        : undefined)
    if (typeof content === 'string') return content
  } catch {
    // 连 JSON 都不是，退回原文
  }
  return body
}

/**
 * 统计一段已累积的 SSE 原始文本里有多少个数据帧。
 *
 * 用于在流式过程中显示「已收到 N 帧」——只累积文本而不给帧数，用户无法判断
 * 是上游在慢慢吐还是已经卡住。
 */
export function countSseFrames(raw: string): number {
  let count = 0
  for (const line of raw.split('\n')) {
    if (line.trim().startsWith('data:')) count += 1
  }
  return count
}
