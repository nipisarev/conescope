import React, { useEffect } from 'react'
import { useAppStore, useProjectStore, useInstanceStore, useSettingsStore } from '@/stores'
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts'
import { NavSidebar } from '@/components/shared/NavSidebar'
import { TopBar } from '@/components/shared/TopBar'
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
  const restoreInstance = useInstanceStore(state => state.restoreInstance)
  const getProject = useProjectStore(state => state.getProject)
  const instances = useInstanceStore(state => state.instances)
  const projectsLoading = useProjectStore(state => state.isLoading)
  const instancesLoading = useInstanceStore(state => state.isLoading)
  const settingsLoading = useSettingsStore(state => state.isLoading)
  const initFromSession = useAppStore(state => state.initFromSession)

  const appendTerminalOutput = useInstanceStore(state => state.appendTerminalOutput)
  const setStatus = useInstanceStore(state => state.setStatus)

  // Track if we've already restored instances
  const restoredRef = React.useRef(false)

  useEffect(() => {
    loadProjects()
    loadInstances()
    loadSettings()

    // Set up global IPC listeners to capture all output
    const cleanupOutput = window.electronAPI.onInstanceOutput((id, data) => {
      appendTerminalOutput(id, data)
    })

    // Also capture shell terminal output (for terminal-only instances)
    const cleanupShellOutput = window.electronAPI.onShellOutput((id, data) => {
      appendTerminalOutput(id, data)
    })

    const cleanupStatus = window.electronAPI.onInstanceStatusChange((id, status) => {
      setStatus(id, status as any)
    })

    return () => {
      cleanupOutput()
      cleanupShellOutput()
      cleanupStatus()
    }
  }, [])

  useKeyboardShortcuts()

  // Initialize app state from session and restore PTY processes
  useEffect(() => {
    if (!projectsLoading && !instancesLoading && !settingsLoading && !restoredRef.current) {
      restoredRef.current = true

      // Initialize view mode and focused instance from session
      initFromSession()

      // Restore each instance's PTY process
      instances.forEach(async (instance) => {
        try {
          if (instance.type === 'terminal') {
            const homePath = await window.electronAPI.getHomePath()
            await window.electronAPI.createShellTerminal(instance.id, homePath)
          } else {
            const project = getProject(instance.projectId!)
            if (project) {
              await restoreInstance(instance.id, project.path)
            }
          }
        } catch (err) {
          console.error('Failed to restore instance:', instance.id, err)
        }
      })
    }
  }, [projectsLoading, instancesLoading, settingsLoading, instances, getProject, restoreInstance, initFromSession])

  // Trigger resize when view mode changes to help terminals re-fit
  useEffect(() => {
    const timeout = setTimeout(() => {
      window.dispatchEvent(new Event('resize'))
    }, 200)
    return () => clearTimeout(timeout)
  }, [viewMode])

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
        <TopBar />
        <div className="app-main">
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
        </div>
        <NavSidebar />
        {newInstanceModalOpen && <NewInstanceModal />}
        {settingsOpen && <SettingsModal />}
      </div>
    </ErrorBoundary>
  )
}
