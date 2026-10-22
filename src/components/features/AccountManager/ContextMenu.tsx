import { memo, useState, useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { useApp } from '../../../hooks/useApp'

interface ContextMenuItem {
  divider?: boolean
  icon?: React.ElementType
  label?: React.ReactNode
  onClick?: () => void
  disabled?: boolean
  danger?: boolean
  shortcut?: string
}

interface ContextMenuProps {
  x: number
  y: number
  onClose: () => void
  items: ContextMenuItem[]
}

// 右键菜单组件（使用 Portal 渲染到 body）
const ContextMenu = memo(function ContextMenu({ x, y, onClose, items }: ContextMenuProps) {
  
  const menuRef = useRef(null)
  const [position, setPosition] = useState({ x, y })

  // 计算菜单位置，确保不超出视口
  useEffect(() => {
    if (!menuRef.current) return
    const menu = menuRef.current
    const rect = menu.getBoundingClientRect()
    const viewportWidth = window.innerWidth
    const viewportHeight = window.innerHeight
    
    let newX = x
    let newY = y
    
    // 右边超出
    if (x + rect.width > viewportWidth - 10) {
      newX = viewportWidth - rect.width - 10
    }
    // 下边超出
    if (y + rect.height > viewportHeight - 10) {
      newY = viewportHeight - rect.height - 10
    }
    
    setPosition({ x: newX, y: newY })
  }, [x, y])

  useEffect(() => {
    const handleClick = () => onClose()
    const handleScroll = () => onClose()
    const handleKeyDown = (e) => e.key === 'Escape' && onClose()
    document.addEventListener('click', handleClick)
    document.addEventListener('keydown', handleKeyDown)
    window.addEventListener('scroll', handleScroll, true)
    return () => {
      document.removeEventListener('click', handleClick)
      document.removeEventListener('keydown', handleKeyDown)
      window.removeEventListener('scroll', handleScroll, true)
    }
  }, [onClose])

  // 使用 Portal 渲染到 body，避免被父元素的 transform 影响
  return createPortal(
    <div
      ref={menuRef}
      className={`fixed z-[9999] min-w-[160px] py-1 rounded-lg shadow-2xl border glass-card border-border`}
      style={{ 
        left: position.x, 
        top: position.y,
        boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.3), 0 10px 10px -5px rgba(0, 0, 0, 0.2)'
      }}
      onClick={(e) => e.stopPropagation()}
    >
      {items.map((item, idx) =>
        item.divider ? (
          <div key={idx} className={`my-2 mx-3 border-t border-border opacity-50`} />
        ) : (
          <button
            key={idx}
            onClick={() => { item.onClick(); onClose() }}
            disabled={item.disabled}
            className={`
              w-full px-2.5 py-1.5 text-left text-sm flex items-center gap-2 
              transition-all duration-200
              disabled:opacity-40 disabled:cursor-not-allowed
              ${item.danger 
                ? 'text-red-400 hover:bg-red-500/15 hover:text-red-300' 
                : `text-foreground hover:bg-muted/50`
              }
            `}
          >
            {item.icon && (
              <div className={`
                w-6 h-6 rounded-md flex items-center justify-center
                ${item.danger ? 'bg-red-500/10' : "bg-muted/30"}
              `}>
                <item.icon 
                  size={14} 
                  className={item.danger ? 'text-red-400' : "text-muted-foreground"}
                  strokeWidth={2}
                />
              </div>
            )}
            <span className="flex-1">{item.label}</span>
            {item.shortcut && (
              <span className={`text-xs px-2 py-0.5 rounded bg-muted/30 text-muted-foreground`}>
                {item.shortcut}
              </span>
            )}
          </button>
        )
      )}
    </div>,
    document.body
  )
})

export default ContextMenu
