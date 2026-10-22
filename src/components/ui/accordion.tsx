import * as React from "react"
import * as AccordionPrimitive from "@radix-ui/react-accordion"
import { ChevronRightIcon } from "lucide-react"

import { cn } from "@/lib/utils"

type AccordionSingleProps = AccordionPrimitive.AccordionSingleProps

type AccordionMultipleProps = AccordionPrimitive.AccordionMultipleProps

type AccordionProps =
  | (Omit<AccordionSingleProps, "type"> & { type?: "single" })
  | AccordionMultipleProps

type AccordionItemProps = AccordionPrimitive.AccordionItemProps

type AccordionTriggerProps = AccordionPrimitive.AccordionTriggerProps

type AccordionContentProps = AccordionPrimitive.AccordionContentProps

function Accordion({
  className,
  ...props
}: AccordionProps) {
  const rootProps: AccordionSingleProps | AccordionMultipleProps =
    props.type === "multiple" ? props : { ...props, type: "single" }

  return (
    <AccordionPrimitive.Root
      {...rootProps}
      data-slot="accordion"
      className={cn("flex w-full flex-col", className)} />
  );
}

function AccordionItem({
  className,
  ...props
}: AccordionItemProps) {
  return (
    <AccordionPrimitive.Item
      data-slot="accordion-item"
      className={cn("not-last:border-b", className)}
      {...props} />
  );
}

function AccordionTrigger({
  className,
  children,
  ...props
}: AccordionTriggerProps) {
  return (
    <AccordionPrimitive.Header className="flex">
      <AccordionPrimitive.Trigger
        data-slot="accordion-trigger"
        className={cn(
          "group/accordion-trigger relative flex flex-1 items-start justify-between gap-2 rounded-lg border border-transparent py-2.5 text-left text-sm font-medium transition-all outline-none hover:underline focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50",
          className
        )}
        {...props}>
        {/* 折叠态用朝右的箭头（▶），展开后旋转 90° 变成朝下（▼）。
            这是折叠列表的通用约定（magpie 的 Agents 行也一样：Details ▶）。

            旋转写在 index.css 里而不是用 Tailwind 变体：实测
            `group-data-[state=open]/accordion-trigger:rotate-90` 生成的
            transform 仍是 none，命名组 + data 变体的组合在这里没命中。 */}
        <ChevronRightIcon
          data-slot="accordion-trigger-icon"
          className="pointer-events-none mt-0.5 size-4 shrink-0 text-muted-foreground transition-transform duration-200" />
        {children}
      </AccordionPrimitive.Trigger>
    </AccordionPrimitive.Header>
  );
}

function AccordionContent({
  className,
  children,
  ...props
}: AccordionContentProps) {
  return (
    <AccordionPrimitive.Content
      data-slot="accordion-content"
      className="overflow-hidden text-sm data-[state=open]:animate-accordion-down data-[state=closed]:animate-accordion-up"
      {...props}>
      <div
        className={cn(
          "h-(--radix-accordion-content-height) pt-0 pb-2.5 [&_a]:underline [&_a]:underline-offset-3 [&_a]:hover:text-foreground [&_p:not(:last-child)]:mb-4",
          className
        )}>
        {children}
      </div>
    </AccordionPrimitive.Content>
  );
}

export { Accordion, AccordionItem, AccordionTrigger, AccordionContent }
