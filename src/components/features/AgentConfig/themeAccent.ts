/**
 * 解析实际生效的主题名。
 *
 * next-themes 在 theme='system' 时会把 `<html data-theme>` 写成解析后的 'light' / 'dark'。
 * 这里读回来，避免 'system' 落到 isLight=true 分支——否则系统切到暗色时，
 * 组件仍会取到亮色 accent / surface 配色，出现浅底浅字的对比度问题。
 */
function resolveTheme(theme: string): string {
  if (theme !== 'system') return theme;
  // next-themes 已把解析后的主题写在 <html data-theme> 上
  return document.documentElement.getAttribute('data-theme') || 'dark';
}

export function getThemeAccent(theme: string) {
  const effective = resolveTheme(theme);
  // 暗色主题列表，须与 index.css 的暗色组（dark / dark-one / tech / midnight）保持一致。
  // 注：forest 的 --background 是 #f0fdf4（亮绿），此前被误列入暗色组，导致取到暗色 accent。
  const isLight = !['dark', 'dark-one', 'tech', 'midnight'].includes(effective);
  
  const accents: Record<string, any> = {
    light: {
      gradientFrom: 'from-blue-400',
      gradientTo: 'to-blue-600',
      shadow: 'shadow-blue-500/20',
      bg: 'bg-blue-500/10',
      bgSoft: 'bg-blue-500/5',
      text: 'text-blue-600',
      textSoft: 'text-blue-500',
      ring: 'focus:ring-blue-500/30',
      border: 'border-blue-500/20',
      borderSoft: 'border-blue-500/10',
      solidBg: 'bg-blue-600',
      scopeBadge: 'bg-blue-500/15 text-blue-600 border border-blue-500/30'
    },
    dark: {
      gradientFrom: 'from-gray-700',
      gradientTo: 'to-gray-900',
      shadow: 'shadow-black/40',
      bg: 'bg-white/10',
      bgSoft: 'bg-white/5',
      text: 'text-white',
      textSoft: 'text-gray-300',
      ring: 'focus:ring-white/20',
      border: 'border-white/20',
      borderSoft: 'border-white/10',
      solidBg: 'bg-white/20',
      scopeBadge: 'bg-white/15 text-white border border-white/30'
    },
    purple: {
      gradientFrom: 'from-purple-500',
      gradientTo: 'to-purple-700',
      shadow: 'shadow-purple-500/20',
      bg: 'bg-purple-500/10',
      bgSoft: 'bg-purple-500/5',
      text: 'text-purple-600',
      textSoft: 'text-purple-500',
      ring: 'focus:ring-purple-500/30',
      border: 'border-purple-500/20',
      borderSoft: 'border-purple-500/10',
      solidBg: 'bg-purple-600',
      scopeBadge: 'bg-purple-500/15 text-purple-600 border border-purple-500/30'
    },
    tech: {
      gradientFrom: 'from-blue-500',
      gradientTo: 'to-cyan-500',
      shadow: 'shadow-cyan-500/20',
      bg: 'bg-cyan-500/10',
      bgSoft: 'bg-cyan-500/5',
      text: 'text-cyan-400',
      textSoft: 'text-cyan-500',
      ring: 'focus:ring-cyan-500/30',
      border: 'border-cyan-500/20',
      borderSoft: 'border-cyan-500/10',
      solidBg: 'bg-cyan-500',
      scopeBadge: 'bg-cyan-500/15 text-cyan-400 border border-cyan-500/30'
    }
  };

  return accents[effective] || accents[isLight ? 'light' : 'dark'];
}

export function getSolidAccentButton(accent: any) {
  return `${accent.solidBg} text-white hover:opacity-90`;
}

export function getGradientAccentButton(accent: any) {
  return `bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} text-white hover:shadow-lg shadow-md`;
}

export function getThemeSurfaceStyles(theme: string) {
  const effective = resolveTheme(theme);
  // 暗色主题列表，须与 index.css 的暗色组（dark / dark-one / tech / midnight）保持一致。
  // 注：forest 的 --background 是 #f0fdf4（亮绿），此前被误列入暗色组，导致取到暗色 accent。
  const isLight = !['dark', 'dark-one', 'tech', 'midnight'].includes(effective);
  return {
    editorBg: isLight ? '#ffffff' : '#1e1e1e',
    editorText: isLight ? '#000000' : '#d4d4d4',
    editorBorder: isLight ? '#e5e7eb' : '#333333',
    inputBg: isLight ? '#ffffff' : '#1e1e1e',
    inputBorder: isLight ? '#e5e7eb' : '#333333',
    inputText: isLight ? '#000000' : '#d4d4d4',
    placeholder: isLight ? '#9ca3af' : '#6b7280',
    dropdownBg: isLight ? '#ffffff' : '#1e1e1e',
    dropdownBorder: isLight ? '#e5e7eb' : '#333333',
    pillBg: isLight ? '#f3f4f6' : '#374151',
    pillText: isLight ? '#1f2937' : '#e5e7eb',
    pillBorder: isLight ? '#e5e7eb' : '#4b5563'
  };
}
