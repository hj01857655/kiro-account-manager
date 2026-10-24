// 项目作用域全局上下文：projectDir 与最近项目列表跨页面共享。
// 持久化到 localStorage——重启恢复上次选择；最近列表（≤5）用于侧边栏快速切换。
import { createContext, useContext, useState, useCallback, type ReactNode } from 'react'

const RECENT_KEY = 'agentRecentProjects'
const CURRENT_KEY = 'agentProjectDir'
const MAX_RECENT = 5

interface ProjectScope {
  projectDir: string | null
  recentDirs: string[]
  setProjectDir: (dir: string | null) => void
}

const ProjectScopeContext = createContext<ProjectScope>({
  projectDir: null,
  recentDirs: [],
  setProjectDir: () => {},
})

export function ProjectScopeProvider({ children }: { children: ReactNode }) {
  const [projectDir, setProjectDirState] = useState<string | null>(() => localStorage.getItem(CURRENT_KEY))
  const [recentDirs, setRecentDirs] = useState<string[]>(() => {
    try { return JSON.parse(localStorage.getItem(RECENT_KEY) || '[]') } catch { return [] }
  })

  const setProjectDir = useCallback((dir: string | null) => {
    setProjectDirState(dir)
    if (dir) {
      localStorage.setItem(CURRENT_KEY, dir)
      setRecentDirs(prev => {
        const next = [dir, ...prev.filter(d => d !== dir)].slice(0, MAX_RECENT)
        localStorage.setItem(RECENT_KEY, JSON.stringify(next))
        return next
      })
    } else {
      localStorage.removeItem(CURRENT_KEY)
    }
  }, [])

  return (
    <ProjectScopeContext.Provider value={{ projectDir, recentDirs, setProjectDir }}>
      {children}
    </ProjectScopeContext.Provider>
  )
}

export function useProjectScope() {
  return useContext(ProjectScopeContext)
}
