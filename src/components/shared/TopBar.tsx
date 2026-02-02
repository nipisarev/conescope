import { useAppStore, useSettingsStore, useInstanceStore, useProjectStore } from '@/stores'
import './TopBar.css'

export function TopBar() {
  const viewMode = useAppStore(state => state.viewMode)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const toggleQuestionsPanel = useSettingsStore(state => state.toggleQuestionsPanel)
  const questionsPanelVisible = useSettingsStore(state => state.questionsPanelVisible)

  const instance = useInstanceStore(state =>
    focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
  )
  const removeInstance = useInstanceStore(state => state.removeInstance)
  const project = useProjectStore(state =>
    instance ? state.getProject(instance.projectId) : undefined
  )

  const handlePause = () => {
    if (focusedInstanceId) {
      window.electronAPI.pauseInstance(focusedInstanceId)
    }
  }

  const handleResume = () => {
    if (focusedInstanceId) {
      window.electronAPI.resumeInstance(focusedInstanceId)
    }
  }

  const handleKill = () => {
    if (focusedInstanceId && confirm('Are you sure you want to kill this instance?')) {
      window.electronAPI.killInstance(focusedInstanceId)
      removeInstance(focusedInstanceId)
      returnToOverview()
    }
  }

  const isFocusMode = viewMode === 'focus' && instance && project

  return (
    <header className="top-bar">
      <div className="top-bar-drag-region" />

      <div className="top-bar-left">
        {isFocusMode ? (
          <>
            <button className="top-bar-back" onClick={returnToOverview} title="Back to Overview (Cmd+0)">
              ←
            </button>
            <span className="top-bar-instance" style={{ color: project.color }}>
              #{instance.instanceNumber}
            </span>
            <span className="top-bar-instance-title">{instance.title}</span>
          </>
        ) : (
          <span className="app-title">◉ Jenklaud</span>
        )}
      </div>

      <div className="top-bar-center">
        {isFocusMode && (
          <div className="instance-controls">
            {instance.status === 'paused' ? (
              <button className="control-btn" onClick={handleResume} title="Resume">
                ▶ Resume
              </button>
            ) : (
              <button className="control-btn" onClick={handlePause} title="Pause">
                ⏸ Pause
              </button>
            )}
            <button className="control-btn danger" onClick={handleKill} title="Kill Instance">
              ✕ Kill
            </button>
          </div>
        )}
      </div>

      <div className="top-bar-right">
        <button className="top-bar-btn primary" onClick={toggleNewInstanceModal}>
          + New
        </button>
        {viewMode === 'overview' && (
          <button
            className={`top-bar-btn ${questionsPanelVisible ? 'active' : ''}`}
            onClick={toggleQuestionsPanel}
          >
            Questions
          </button>
        )}
      </div>
    </header>
  )
}
