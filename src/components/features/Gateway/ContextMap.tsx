import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

export interface ContextPart {
  kind: string
  tokens: number
}

export interface ContextMapProps {
  /** 各类构成（kind → tokens），来自 prompt 构成估算 */
  parts: ContextPart[]
  /** 该请求的 prompt 总 tokens */
  tokens: number
  /** 上下文窗口大小（tokens）；未知时为 null——不画空闲区，也不猜 */
  window: number | null
  className?: string
}

/** 地图上各类构成的配色（与 request_info 图表的色系对齐）。 */
const PART_COLORS: Record<string, { bg: string; text: string }> = {
  system: { bg: 'rgba(59,130,246,0.85)', text: '#fff' },
  tools: { bg: 'rgba(168,85,247,0.85)', text: '#fff' },
  skills: { bg: 'rgba(236,72,153,0.85)', text: '#fff' },
  memory: { bg: 'rgba(245,158,11,0.85)', text: '#1f2937' },
  files: { bg: 'rgba(16,185,129,0.85)', text: '#fff' },
  results: { bg: 'rgba(239,68,68,0.85)', text: '#fff' },
  chat: { bg: 'rgba(100,116,139,0.85)', text: '#fff' },
}

const FALLBACK_COLOR = { bg: 'rgba(148,163,184,0.85)', text: '#fff' }

const colorOf = (kind: string) => PART_COLORS[kind] ?? FALLBACK_COLOR

/** 一块地图瓦片：矩形 + 构成类别 + tokens。 */
export interface Tile {
  kind: string
  tokens: number
  x: number
  y: number
  w: number
  h: number
}

/**
 * squarify 布局（Bruls / Huizing / van Wijk）：把带权矩形排进给定区域，
 * 让每块的宽高尽量接近正方形。
 *
 * 坐标是**百分比**（0–100），与容器实际像素解耦——容器缩放不必重算。
 */
export function squarify(
  vals: Array<{ v: number; kind: string }>,
  x0: number,
  y0: number,
  w0: number,
  h0: number,
): Tile[] {
  const out: Tile[] = []
  const items = vals.filter((a) => a.v > 0)
  const total = items.reduce((s, a) => s + a.v, 0)
  if (total <= 0 || w0 <= 0 || h0 <= 0) return out

  // 权重 → 面积的缩放系数
  const k = (w0 * h0) / total

  // 一行瓦片的最差长宽比（越小越方正）
  const worst = (row: Array<{ v: number }>, side: number): number => {
    let sum = 0
    let hi = 0
    let lo = Infinity
    for (const a of row) {
      const s = a.v * k
      sum += s
      hi = Math.max(hi, s)
      lo = Math.min(lo, s)
    }
    if (sum <= 0 || lo <= 0) return Infinity
    return Math.max((side * side * hi) / (sum * sum), (sum * sum) / (side * side * lo))
  }

  // 剩余待铺区域（随铺随减）
  let x = x0
  let y = y0
  let w = w0
  let h = h0

  // 把一行铺进当前剩余区域
  const lay = (row: Array<{ v: number; kind: string }>) => {
    const sum = row.reduce((s, a) => s + a.v * k, 0)
    if (w >= h) {
      const cw = sum / h
      let cy = y
      for (const a of row) {
        const ch = (a.v * k) / cw
        out.push({ kind: a.kind, tokens: a.v, x, y: cy, w: cw, h: ch })
        cy += ch
      }
      x += cw
      w -= cw
    } else {
      const rh = sum / w
      let cx = x
      for (const a of row) {
        const cw = (a.v * k) / rh
        out.push({ kind: a.kind, tokens: a.v, x: cx, y, w: cw, h: rh })
        cx += cw
      }
      y += rh
      h -= rh
    }
  }

  let row: Array<{ v: number; kind: string }> = []
  for (const a of items) {
    const side = Math.min(w, h)
    if (row.length === 0 || worst([...row, a], side) <= worst(row, side)) {
      row.push(a)
      continue
    }
    lay(row)
    row = [a]
  }
  if (row.length > 0) lay(row)
  return out
}

/** 单块瓦片的最小面积占比（占已用区域）：太小的块画出来也看不清。 */
const TILE_FLOOR = 0.02

/**
 * 上下文构成地图：把窗口按「谁占满了它」画成瓦片。
 *
 * 已用部分与空闲部分沿长边并排：空闲画成斜纹块，占用比例一目了然。
 * 已用区域的**长度按幂曲线缓动**（share^0.35）——窗口只用了 8% 时
 * 仍给已用部分约四成画面，小块才看得清；空闲块用自己的文案写明真实占比。
 */
export function ContextMap({ parts, tokens, window, className = '' }: ContextMapProps) {
  // 纯展示组件，只取翻译——不必挂 Dialog/Settings 等一堆 context
  const { t } = useTranslation()

  const view = useMemo(() => {
    const used = parts.filter((p) => p.tokens > 0).map((p) => ({ v: p.tokens, kind: p.kind }))
    const totalUsed = used.reduce((s, p) => s + p.v, 0)
    // 窗口未知：不知道还剩多少，就不画空闲区（也不猜一个窗口）
    const scale = window != null && window > 0 ? Math.max(window, totalUsed) : totalUsed
    const free = window != null ? Math.max(0, scale - totalUsed) : 0

    if (totalUsed <= 0) return null

    // 已用区域在长边上的长度：满窗时占满；未满时缓动，并保底 30%
    // 缓动的理由（share^0.35）：窗口只用了 8% 时仍给已用部分约四成画面，
    // 小块才看得清；反过来，窗口快满时（如 90%）空闲区会只剩一条窄缝，
    // 「还剩多少」反而看不清——所以空闲区的宽度也按同样曲线保底。
    const usedRatio = scale > 0 ? Math.min(1, totalUsed / scale) : 0
    const usedLen = usedRatio >= 1 ? 100 : Math.max(30, 100 * Math.pow(usedRatio, 0.35))

    // 极小的块保底面积，否则只剩一条线，看不出是什么
    const floored = used.map((a) => ({
      ...a,
      v: Math.max(a.v, (totalUsed * TILE_FLOOR) / used.length),
    }))

    const tiles = squarify(floored, 0, 0, usedLen, 100)
    return { tiles, free, scale, usedLen, totalUsed }
  }, [parts, window])

  if (!view) {
    return (
      <div className={`flex items-center justify-center rounded-lg border border-dashed border-border text-xs text-muted-foreground h-32 ${className}`}>
        {t('gatewayContext.mapEmpty')}
      </div>
    )
  }

  const { tiles, free, scale, totalUsed, usedLen } = view

  return (
    <div className={`space-y-2 ${className}`}>
      <div className="relative w-full max-w-[440px] aspect-square rounded-lg overflow-hidden border border-border bg-muted/20">
        {tiles.map((tile) => {
          const c = colorOf(tile.kind)
          const label = t(`gatewayContext.part.${tile.kind}`)
          // 块太小时不写文字，只留 title 提示
          const roomy = tile.w > 11 && tile.h > 7
          return (
            <div
              key={tile.kind}
              className="absolute overflow-hidden"
              style={{
                left: `${tile.x}%`,
                top: `${tile.y}%`,
                width: `${tile.w}%`,
                height: `${tile.h}%`,
                background: c.bg,
              }}
              title={`${label} · ${fmtK(tile.tokens)}`}
            >
              {roomy && (
                <div className="px-1.5 py-1 text-[10px] leading-tight" style={{ color: c.text }}>
                  <div className="font-medium truncate">{label}</div>
                  <div className="tabular-nums opacity-90">{fmtK(tile.tokens)}</div>
                </div>
              )}
            </div>
          )
        })}

        {free > 0 && usedLen < 99.5 && (
          <div
            className="absolute overflow-hidden"
            style={{
              left: `${usedLen}%`,
              top: 0,
              width: `${100 - usedLen}%`,
              height: '100%',
              background:
                'repeating-linear-gradient(45deg, rgba(148,163,184,0.20) 0 6px, rgba(148,163,184,0.08) 6px 12px)',
            }}
            title={`${t('gatewayContext.mapFree')} · ${fmtK(free)}`}
          >
            {/* 空闲区太窄时不写文字——写进去会被压成竖条溢出块外 */}
            {100 - usedLen > 16 && (
              <div className="px-1.5 py-1 text-[10px] leading-tight text-muted-foreground">
                <div className="font-medium">{t('gatewayContext.mapFree')}</div>
                <div className="tabular-nums opacity-80">
                  {fmtK(free)} · {Math.round((free / scale) * 100)}%
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* 图例：各类占已用部分的比例（空闲区单列） */}
      <div className="flex flex-wrap gap-x-3 gap-y-1 text-[10px]">
        {parts.filter((p) => p.tokens > 0).map((p) => (
          <span key={p.kind} className="inline-flex items-center gap-1 text-muted-foreground">
            <span className="w-2 h-2 rounded-sm" style={{ background: colorOf(p.kind).bg }} />
            {t(`gatewayContext.part.${p.kind}`)}
            <span className="tabular-nums">{Math.round((p.tokens / totalUsed) * 100)}%</span>
          </span>
        ))}
        {free > 0 && (
          <span className="inline-flex items-center gap-1 text-muted-foreground">
            <span className="w-2 h-2 rounded-sm bg-muted" />
            {t('gatewayContext.mapFree')}
            <span className="tabular-nums">{Math.round((free / scale) * 100)}%</span>
          </span>
        )}
      </div>
    </div>
  )
}

function fmtK(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10_000 ? 0 : 1)}K`
  return String(Math.round(n))
}
