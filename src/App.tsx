import { useEffect } from 'react'
import { useAppStore, useProjectStore, useInstanceStore, useSettingsStore } from '@/stores'
import { TopBar } from '@/components/shared/TopBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { InboxPanel } from '@/components/Overview/InboxPanel'
import { FocusView } from '@/components/Focus/FocusView'
import { NewInstanceModal } from '@/components/shared/NewInstanceModal'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)
  const newInstanceModalOpen = useAppStore(state => state.newInstanceModalOpen)

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

  const isLoading = projectsLoading || instancesLoading || settingsLoading

  if (isLoading) {
    return (
      <div className="app-loading">
        <div className="loading-spinner">Loading...</div>
      </div>
    )
  }

  if (viewMode === 'focus') {
    return (
      <>
        <FocusView />
        {newInstanceModalOpen && <NewInstanceModal />}
      </>
    )
  }

  return (
    <div className="app-container">
      <TopBar />
      <div className="main-content">
        <OverviewGrid />
        <InboxPanel />
      </div>
      {newInstanceModalOpen && <NewInstanceModal />}
    </div>
  )
}
