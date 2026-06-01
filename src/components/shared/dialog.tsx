import * as React from "react"
import { Dialog as HeadlessDialog, DialogPanel, DialogTitle, Description, DialogBackdrop, CloseButton } from '@headlessui/react'
import { X, LucideIcon } from "lucide-react"
import { cn } from "../../lib/utils"

interface DialogRootProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: React.ReactNode;
}

/**
 * DialogRoot - dialog root component.
 */
const DialogRoot = ({ open, onOpenChange, children }: DialogRootProps) => {
  return (
    <HeadlessDialog 
      open={open}
      onClose={() => onOpenChange?.(false)}
      className="relative z-50"
    >
      {children}
    </HeadlessDialog>
  )
}

interface DialogOverlayProps {
  className?: string;
}

/**
 * DialogOverlay - backdrop overlay.
 */
const DialogOverlay = React.forwardRef<HTMLDivElement, DialogOverlayProps>(({ className, ...props }, ref) => {
  return (
    <DialogBackdrop
      ref={ref}
      transition
      className={cn(
        "fixed inset-0",
        "bg-black/60 backdrop-blur-sm",
        "duration-300 ease-out",
        "data-[closed]:opacity-0",
        className
      )}
      {...props}
    />
  )
})
DialogOverlay.displayName = "DialogOverlay"

interface DialogContentProps {
  className?: string;
  children: React.ReactNode;
  maxWidth?: string;
  showClose?: boolean;
}

/**
 * DialogContent - 弹窗内容容器
 */
const DialogContent = React.forwardRef<HTMLDivElement, DialogContentProps>(({ 
  className, 
  children, 
  maxWidth = "400px",
  showClose = true,
  ...props 
}, ref) => {
  return (
    <>
      <DialogOverlay />
      
      <div className="fixed inset-0 flex items-center justify-center p-4">
        <DialogPanel
          ref={ref}
          transition
          className={cn(
            "relative w-full max-h-[90vh]",
            "flex flex-col",
            "rounded-2xl border shadow-2xl",
            "duration-300 ease-out",
            "data-[closed]:opacity-0 data-[closed]:scale-95",
            "glass-card",
            "border-border",
            className
          )}
          style={{ maxWidth }}
          {...props}
        >
          {children}
          {showClose && (
            <CloseButton
              className={cn(
                "absolute right-4 top-4 z-10",
                "p-2 rounded-xl",
                "transition-all duration-200",
                "hover:scale-110",
                "focus:outline-none focus:ring-2 focus:ring-blue-500/30",
                "hover:bg-muted/50"
              )}
            >
              <X size={18} className="text-muted-foreground" />
              <span className="sr-only">Close</span>
            </CloseButton>
          )}
        </DialogPanel>
      </div>
    </>
  )
})
DialogContent.displayName = "DialogContent"

interface DialogHeaderProps {
  className?: string;
  icon?: LucideIcon;
  iconColor?: string;
  iconBg?: string;
  children: React.ReactNode;
}

/**
 * DialogHeader - 弹窗头部
 */
const DialogHeader = React.forwardRef<HTMLDivElement, DialogHeaderProps>(({ 
  className, 
  icon: Icon, 
  iconColor, 
  iconBg, 
  children, 
  ...props 
}, ref) => {
  return (
    <div
      ref={ref}
      className={cn("px-6 pt-6 pb-2", className)}
      {...props}
    >
      {Icon && (
        <div className="flex items-center gap-3 mb-2">
          <div className={cn(
            "w-10 h-10 rounded-xl",
            "flex items-center justify-center",
            "shadow-md",
            iconBg || "bg-gradient-to-br from-blue-500/20 to-indigo-500/10"
          )}>
            <Icon 
              size={20} 
              className={iconColor || "text-blue-400"} 
              strokeWidth={2} 
            />
          </div>
        </div>
      )}
      {children}
    </div>
  )
})
DialogHeader.displayName = "DialogHeader"

interface DialogTitleProps {
  className?: string;
  children: React.ReactNode;
}

/**
 * DialogTitle - 弹窗标题
 */
const DialogTitleComponent = React.forwardRef<HTMLHeadingElement, DialogTitleProps>(({ className, ...props }, ref) => {
  return (
    <DialogTitle
      ref={ref}
      className={cn(
        "text-lg font-semibold leading-tight",
        "text-foreground",
        className
      )}
      {...props}
    />
  )
})
DialogTitleComponent.displayName = "DialogTitle"

interface DialogDescriptionProps {
  className?: string;
  children: React.ReactNode;
}

/**
 * DialogDescription - 弹窗描述
 */
const DialogDescriptionComponent = React.forwardRef<HTMLParagraphElement, DialogDescriptionProps>(({ className, ...props }, ref) => {
  return (
    <Description
      ref={ref}
      className={cn("text-sm mt-1", "text-muted-foreground", className)}
      {...props}
    />
  )
})
DialogDescriptionComponent.displayName = "DialogDescription"

interface DialogBodyProps {
  className?: string;
  children: React.ReactNode;
  gap?: 'none' | 'sm' | 'md' | 'lg' | 'xl';
  noPadding?: boolean;
}

/**
 * DialogBody - 弹窗内容区域
 */
const DialogBody = React.forwardRef<HTMLDivElement, DialogBodyProps>(({ 
  className, 
  gap = "md",
  noPadding = false,
  ...props 
}, ref) => {
  const gapClasses = {
    none: "",
    sm: "space-y-3",
    md: "space-y-4",
    lg: "space-y-6",
    xl: "space-y-8"
  }
  
  return (
    <div
      ref={ref}
      className={cn(
        !noPadding && "px-6 py-4",
        "overflow-y-auto flex-1 no-scrollbar",
        gapClasses[gap],
        className
      )}
      style={{ scrollbarWidth: 'thin' }}
      {...props}
    />
  )
})
DialogBody.displayName = "DialogBody"

interface DialogFooterProps {
  className?: string;
  children: React.ReactNode;
}

/**
 * DialogFooter - 弹窗底部
 */
const DialogFooter = React.forwardRef<HTMLDivElement, DialogFooterProps>(({ className, ...props }, ref) => {
  return (
    <div
      ref={ref}
      className={cn(
        "px-6 py-4",
        "flex justify-end gap-3",
        "bg-muted/10 border-t border-border/30",
        className
      )}
      {...props}
    />
  )
})
DialogFooter.displayName = "DialogFooter"

/**
 * DialogClose - 关闭按钮
 */
const DialogClose = CloseButton

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title?: string;
  description?: string;
  children?: React.ReactNode;
  footer?: React.ReactNode;
  maxWidth?: string;
  icon?: LucideIcon;
  iconColor?: string;
  iconBg?: string;
  showClose?: boolean;
}

/**
 * Dialog - 完整的对话框组件
 */
export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  maxWidth = '400px',
  icon: Icon,
  iconColor,
  iconBg,
  showClose = true
}: DialogProps) {
  return (
    <DialogRoot open={open} onOpenChange={onOpenChange}>
      <DialogContent maxWidth={maxWidth} showClose={showClose}>
        {(title || description || Icon) && (
          <DialogHeader icon={Icon} iconColor={iconColor} iconBg={iconBg}>
            {title && <DialogTitleComponent>{title}</DialogTitleComponent>}
            {description && <DialogDescriptionComponent>{description}</DialogDescriptionComponent>}
          </DialogHeader>
        )}
        
        {children && (
          <DialogBody>{children}</DialogBody>
        )}
        
        {footer && (
          <DialogFooter>{footer}</DialogFooter>
        )}
      </DialogContent>
    </DialogRoot>
  )
}

export {
  DialogRoot,
  DialogOverlay,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitleComponent as DialogTitle,
  DialogDescriptionComponent as DialogDescription,
  DialogBody
}
