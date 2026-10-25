import { Card } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'

/** 首页骨架屏：与实际 Home 布局结构对齐，避免内容到位时的视觉跳动。 */
function LoadingSkeleton() {
  return (
    <div className="h-full overflow-auto surface-main">
      <div className="w-full p-5">
        {/* Header */}
        <div className="flex items-center gap-2.5 mb-4">
          <Skeleton className="w-10 h-10 rounded-xl" />
          <div className="flex flex-col gap-1.5">
            <Skeleton className="w-32 h-4" />
            <Skeleton className="w-48 h-3" />
          </div>
        </div>

        {/* 统计卡片 5 列 */}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3 mb-4">
          {[...Array(5)].map((_, i) => (
            <Card key={i} className="card-glow rounded-xl">
              <div className="flex gap-3 items-center p-4">
                <Skeleton className="w-9 h-9 rounded-lg" />
                <div className="flex flex-col gap-1.5 flex-1">
                  <Skeleton className="w-1/2 h-5" />
                  <Skeleton className="w-3/4 h-3" />
                </div>
              </div>
            </Card>
          ))}
        </div>

        {/* 主卡片：当前 IDE | CLI 双栏 */}
        <Card className="card-glow rounded-xl">
          <div className="px-4 py-3 border-b border-border">
            <Skeleton className="w-32 h-4" />
          </div>
          <div className="grid grid-cols-1 md:grid-cols-[3fr_2fr]">
            <div className="p-5 flex flex-col gap-3">
              <Skeleton className="w-24 h-3" />
              <Skeleton className="w-full h-14 rounded-xl" />
              <Skeleton className="w-full h-20 rounded-xl" />
            </div>
            <div className="p-5 flex flex-col gap-3 bg-muted/20 border-l border-border">
              <Skeleton className="w-24 h-3" />
              <Skeleton className="w-full h-14 rounded-xl" />
              <Skeleton className="w-full h-20 rounded-xl" />
            </div>
          </div>
        </Card>
      </div>
    </div>
  )
}

export default LoadingSkeleton
