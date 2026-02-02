import { useCallback } from 'react'
import { useInstanceStore, useProjectStore, useAppStore } from '@/stores'
import { Terminal } from './Terminal'
import './FocusView.css'

export function FocusView() {
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const instance = useInstanceStore(state =>
    focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
  )
  const project = useProjectStore(state =>
    instance ? state.getProject(instance.projectId) : undefined
  )

  const handleInput = useCallback((data: string) => {
    if (focusedInstanceId) {
      window.electronAPI.sendInput(focusedInstanceId, data)
    }
  }, [focusedInstanceId])

  const handlePause = () => {
    if (focusedInstanceId) {
      window.electronAPI.pauseInstance(focusedInstanceId)
    }
  }

  const handleKill = () => {
    if (focusedInstanceId && confirm('Are you sure you want to kill this instance?')) {
      window.electronAPI.killInstance(focusedInstanceId)
      returnToOverview()
    }
  }

  if (!instance || !project) {
    return (
      <div className="focus-view-empty">
        <p>No instance selected</p>
        <button onClick={returnToOverview}>Return to Overview</button>
      </div>
    )
  }

  const duration = Math.floor(
    (Date.now() - new Date(instance.startedAt).getTime()) / 1000 / 60
  )

  return (
    <div className="focus-view">
      <div className="focus-header" style={{ borderBottomColor: project.color }}>
        <button className="back-btn" onClick={returnToOverview}>
          ← Back to Overview
        </button>
        <span className="focus-project-name">{project.displayName}</span>
        <div className="focus-actions">
          {instance.status === 'paused' ? (
            <button
              className="action-btn"
              onClick={() => window.electronAPI.resumeInstance(focusedInstanceId!)}
            >
              Resume
            </button>
          ) : (
            <button className="action-btn" onClick={handlePause}>
              Pause
            </button>
          )}
          <button className="action-btn action-btn-danger" onClick={handleKill}>
            Kill
          </button>
        </div>
      </div>

      <div className="focus-content">
        <div className="focus-sidebar">
          <div className="sidebar-section">
            <h3>Files</h3>
            <p className="placeholder">File explorer coming soon</p>
          </div>
        </div>

        <div className="focus-main">
          <div className="focus-editor">
            <p className="placeholder">Monaco Editor coming soon</p>
          </div>

          <div className="focus-terminal">
            <Terminal instanceId={instance.id} onInput={handleInput} />
          </div>
        </div>
      </div>

      <div className="focus-stats-bar">
        <span>Stats: {(instance.tokensUsed / 1000).toFixed(0)}k tokens</span>
        <span className="stats-divider">│</span>
        <span>${instance.costEstimate.toFixed(2)}</span>
        <span className="stats-divider">│</span>
        <span>{duration} min</span>
        <span className="stats-divider">│</span>
        <span>Git Changes: 0</span>
      </div>
    </div>
  )
}
