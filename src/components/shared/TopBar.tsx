import { useState } from 'react'
import {
  useAppStore,
  useSettingsStore,
  useInstanceStore,
  useProjectStore,
} from '@/stores'
import { Settings, MinusCircleSolid, XmarkCircleSolid } from 'iconoir-react'
import { CloseConfirmModal } from './CloseConfirmModal'
import './TopBar.css'

export function TopBar() {
  const viewMode = useAppStore(state => state.viewMode)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const toggleSettings = useAppStore(state => state.toggleSettings)
  const toggleQuestionsPanel = useSettingsStore(state => state.toggleQuestionsPanel)
  const questionsPanelVisible = useSettingsStore(state => state.questionsPanelVisible)

  const instance = useInstanceStore(state =>
    focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
  )
  const removeInstance = useInstanceStore(state => state.removeInstance)
  const project = useProjectStore(state =>
    instance ? state.getProject(instance.projectId) : undefined
  )

  const [showCloseModal, setShowCloseModal] = useState(false)

  const handleMinimize = () => {
    returnToOverview()
  }

  const handleCloseClick = () => {
    setShowCloseModal(true)
  }

  const handleCloseConfirm = () => {
    if (focusedInstanceId) {
      window.electronAPI.killInstance(focusedInstanceId)
      removeInstance(focusedInstanceId)
      returnToOverview()
    }
    setShowCloseModal(false)
  }

  const handleCloseCancel = () => {
    setShowCloseModal(false)
  }

  const isFocusMode = viewMode === 'focus' && instance && project

  return (
    <>
      <header className="top-bar">
        <div className="top-bar-drag-region" />

        <div className="top-bar-left" />

        <div className="top-bar-center">
          {isFocusMode ? (
            <span className="app-title" style={{ color: project.color }}>
              #{instance.instanceNumber} {instance.title.toUpperCase()}
            </span>
          ) : (
            <span className="app-title">JENKLAUD</span>
          )}
        </div>

        <div className="top-bar-right">
          {isFocusMode ? (
            <div className="instance-controls">
              <button
                className="control-btn icon-btn"
                onClick={handleMinimize}
                title="Minimize (return to overview)"
              >
                <MinusCircleSolid width={18} height={18} />
              </button>
              <button
                className="control-btn icon-btn"
                onClick={handleCloseClick}
                title="Close Instance"
              >
                <XmarkCircleSolid width={18} height={18} />
              </button>
            </div>
          ) : (
            <>
              <button
                className="top-bar-btn primary"
                onClick={toggleNewInstanceModal}
              >
                + New
              </button>
              <button
                className={`top-bar-btn ${questionsPanelVisible ? 'active' : ''}`}
                onClick={toggleQuestionsPanel}
              >
                Questions
              </button>
              <button
                className="top-bar-btn icon-btn"
                onClick={toggleSettings}
                title="Settings"
              >
                <Settings width={18} height={18} />
              </button>
            </>
          )}
        </div>
      </header>

      {showCloseModal && instance && project && (
        <CloseConfirmModal
          instanceNumber={instance.instanceNumber}
          instanceTitle={instance.title}
          projectColor={project.color}
          status={instance.status}
          onConfirm={handleCloseConfirm}
          onCancel={handleCloseCancel}
        />
      )}
    </>
  )
}
