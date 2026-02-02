import { useEffect } from 'react'
import { useAppStore, useProjectStore, useInstanceStore, useSettingsStore } from '@/stores'
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts'
import { NavSidebar } from '@/components/shared/NavSidebar'
import { TopBar } from '@/components/shared/TopBar'
import { StatusBar } from '@/components/shared/StatusBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { QuestionsPanel } from '@/components/Overview/QuestionsPanel'
import { FocusView } from '@/components/Focus/FocusView'
import { NewInstanceModal } from '@/components/shared/NewInstanceModal'
import { SettingsModal } from '@/components/shared/SettingsModal'
import { ErrorBoundary } from '@/components/shared/ErrorBoundary'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)
  const newInstanceModalOpen = useAppStore(state => state.newInstanceModalOpen)
  const settingsOpen = useAppStore(state => state.settingsOpen)
  const questionsPanelVisible = useSettingsStore(state => state.questionsPanelVisible)

  const loadProjects = useProjectStore(state => state.loadProjects)
  const loadInstances = useInstanceStore(state => state.loadInstances)
  const loadSettings = useSettingsStore(state => state.loadSettings)
  const projectsLoading = useProjectStore(state => state.isLoading)
  const instancesLoading = useInstanceStore(state => state.isLoading)
  const settingsLoading = useSettingsStore(state => state.isLoading)

  useEffect(() => {
    loadProjects()
    loadInstances()
    loadSettings()
  }, [])

  useKeyboardShortcuts()

  const isLoading = projectsLoading || instancesLoading || settingsLoading

  if (isLoading) {
    return (
      <div className="app-loading">
        <div className="loading-spinner">Loading...</div>
      </div>
    )
  }

  return (
    <ErrorBoundary>
      <div className="app-container">
        <NavSidebar />
        <div className="app-main">
          <TopBar />
          <div className="app-content">
            {viewMode === 'focus' ? (
              <FocusView />
            ) : (
              <>
                <OverviewGrid />
                {questionsPanelVisible && <QuestionsPanel />}
              </>
            )}
          </div>
          <StatusBar />
        </div>
        {newInstanceModalOpen && <NewInstanceModal />}
        {settingsOpen && <SettingsModal />}
      </div>
    </ErrorBoundary>
  )
}
