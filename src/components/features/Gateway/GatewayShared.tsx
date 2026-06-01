import type React from 'react'
import { Card, CardContent } from '@/components/ui/card'

interface GatewaySurfaceCardProps extends React.HTMLAttributes<HTMLDivElement> {}

export function GatewaySurfaceCard({ className = '', children, ...props }: GatewaySurfaceCardProps) {
  return (
    <Card
      className={`glass-card border-border border rounded-md ${className}`.trim()}
      {...props}
    >
      <CardContent className="p-3">
        {children}
      </CardContent>
    </Card>
  )
}

interface GatewaySubCardProps extends React.HTMLAttributes<HTMLDivElement> {
  children?: React.ReactNode;
}

export function GatewaySubCard({ className = '', children, ...props }: GatewaySubCardProps) {
  return (
    <Card className={`border rounded-md ${className}`} {...props}>
      <CardContent className="p-3">
        {children}
      </CardContent>
    </Card>
  )
}

interface GatewaySectionHeaderProps {
  colors?: Record<string, string>;
  icon?: React.ComponentType<{ size?: number }>;
  title: string;
  badge?: React.ReactNode;
  actions?: React.ReactNode;
  groupProps?: React.HTMLAttributes<HTMLDivElement>;
}

export function GatewaySectionHeader({ colors, icon: Icon, title, badge, actions, groupProps = {} }: GatewaySectionHeaderProps) {
  return (
    <div className="flex items-center justify-between" {...groupProps}>
      <div className="flex items-center gap-2">
        {Icon ? <Icon size={16} /> : null}
        <h3 className={`font-semibold text-foreground`}>{title}</h3>
      </div>
      {actions || badge || null}
    </div>
  )
}

interface GatewayStatCardProps {
  colors?: Record<string, string>;
  label: string;
  value: string | number;
  detail?: string;
  valueProps?: React.HTMLAttributes<HTMLParagraphElement>;
  className?: string;
}

export function GatewayStatCard({ colors, label, value, detail, valueProps = {}, className = '' }: GatewayStatCardProps) {
  return (
    <GatewaySubCard className={className}>
      <p className={`text-xs text-muted-foreground`}>{label}</p>
      <p className={`font-bold mt-1 text-foreground`} {...valueProps}>
        {value}
      </p>
      {detail ? (
        <p className={`text-sm mt-1.5 text-muted-foreground`}>
          {detail}
        </p>
      ) : null}
    </GatewaySubCard>
  )
}

interface GatewayPathCardProps {
  title?: string;
  value: string;
  actions?: React.ReactNode;
}

export function GatewayPathCard({ title = 'Log Directory', value, actions }: GatewayPathCardProps) {
  return (
    <GatewaySubCard>
      <p className="text-xs font-semibold">{title}</p>
      <p className="text-xs mt-1.5 font-mono break-all">
        {value}
      </p>
      {actions ? (
        <div className="mt-3 flex items-center gap-2">
          {actions}
        </div>
      ) : null}
    </GatewaySubCard>
  )
}

interface GatewayCodeCardProps {
  title?: string;
  code?: string;
  description?: string;
  actions?: React.ReactNode;
  children?: React.ReactNode;
}

export function GatewayCodeCard({ title, code, description, actions, children }: GatewayCodeCardProps) {
  return (
    <GatewaySubCard>
      {title ? <p className="text-xs font-semibold">{title}</p> : null}
      {code ? (
        <pre className={`bg-muted rounded-md p-3 overflow-x-auto text-xs font-mono ${title ? 'mt-2' : ''}`}>
          <code>{code}</code>
        </pre>
      ) : null}
      {description ? (
        <p className="text-xs mt-2">
          {description}
        </p>
      ) : null}
      {children || null}
      {actions ? (
        <div className="mt-3 flex items-center gap-2">
          {actions}
        </div>
      ) : null}
    </GatewaySubCard>
  )
}
