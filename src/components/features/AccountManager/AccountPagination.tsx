import { ChevronsLeft, ChevronLeft, ChevronRight, ChevronsRight } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'

function AccountPagination({
  totalCount,
  pageSize,
  currentPage,
  totalPages,
  onPageSizeChange,
  onPageChange}) {
  const { t, theme} = useApp()

  if (totalCount === 0) return null

  return (
    <div className={`surface-card border-t border-border px-6 py-3 flex items-center justify-between animate-fade-in delay-300`}>
      <div className={`flex items-center gap-2 text-sm text-muted-foreground`}>
        <span>{t('pagination.perPage')}</span>
        <select
          value={pageSize}
          onChange={(e) => onPageSizeChange(Number(e.target.value))}
          className={`px-2 py-1 border rounded-lg surface-card border-border text-sm text-foreground`}
        >
          <option value={10}>10</option>
          <option value={20}>20</option>
          <option value={50}>50</option>
        </select>
        <span>{t('pagination.totalItems', { count: totalCount })}</span>
      </div>
      <div className="flex items-center gap-1">
        <button onClick={() => onPageChange(1)} disabled={currentPage === 1} className={`p-2 border border-border rounded-lg hover:bg-muted/50 disabled:opacity-40`} title={t('pagination.first')}>
          <ChevronsLeft size={16} className={"text-muted-foreground"} />
        </button>
        <button onClick={() => onPageChange(currentPage - 1)} disabled={currentPage === 1} className={`p-2 border border-border rounded-lg hover:bg-muted/50 disabled:opacity-40`} title={t('pagination.prev')}>
          <ChevronLeft size={16} className={"text-muted-foreground"} />
        </button>
        <span className={`px-4 py-1.5 text-sm text-foreground font-medium`}>{currentPage} / {totalPages}</span>
        <button onClick={() => onPageChange(currentPage + 1)} disabled={currentPage === totalPages} className={`p-2 border border-border rounded-lg hover:bg-muted/50 disabled:opacity-40`} title={t('pagination.next')}>
          <ChevronRight size={16} className={"text-muted-foreground"} />
        </button>
        <button onClick={() => onPageChange(totalPages)} disabled={currentPage === totalPages} className={`p-2 border border-border rounded-lg hover:bg-muted/50 disabled:opacity-40`} title={t('pagination.last')}>
          <ChevronsRight size={16} className={"text-muted-foreground"} />
        </button>
      </div>
    </div>
  )
}

export default AccountPagination
