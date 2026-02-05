import { useState } from 'react'
import {
  useAppStore,
  useSettingsStore,
  useInstanceStore,
  useProjectStore,
  useTerminalTabStore,
} from '@/stores'
import {
  Settings,
  MinusCircle,
  MinusCircleSolid,
  XmarkCircle,
  XmarkCircleSolid,
  ChatBubbleQuestion,
  ChatBubbleQuestionSolid,
  Plus,
} from 'iconoir-react'
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
  const getDisplayNumber = useInstanceStore(state => state.getDisplayNumber)
  const removeInstance = useInstanceStore(state => state.removeInstance)
  const cleanupTerminalTabs = useTerminalTabStore(state => state.cleanupInstance)
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
      cleanupTerminalTabs(focusedInstanceId)
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
              #{getDisplayNumber(instance.id)} {instance.title.toUpperCase()}
            </span>
          ) : (
            <span className="app-title">CONESCOPE</span>
          )}
        </div>

        <div className="top-bar-right">
          {isFocusMode ? (
            <div className="instance-controls">
              <button
                className="control-btn icon-btn minimize-btn"
                onClick={handleMinimize}
                title="Minimize (return to overview)"
              >
                <MinusCircle width={18} height={18} className="icon-outline" />
                <MinusCircleSolid width={18} height={18} className="icon-solid" />
              </button>
              <button
                className="control-btn icon-btn close-btn"
                onClick={handleCloseClick}
                title="Close Instance"
              >
                <XmarkCircle width={18} height={18} className="icon-outline" />
                <XmarkCircleSolid width={18} height={18} className="icon-solid" />
              </button>
            </div>
          ) : (
            <>
              <button
                className="new-window-btn"
                onClick={toggleNewInstanceModal}
              >
                <Plus width={14} height={14} />
                New Window
              </button>
              <button
                className={`icon-toggle-btn ${questionsPanelVisible ? 'active' : ''}`}
                onClick={toggleQuestionsPanel}
                title="Questions"
              >
                <ChatBubbleQuestion width={18} height={18} className="icon-outline" />
                <ChatBubbleQuestionSolid width={18} height={18} className="icon-solid" />
              </button>
              <button
                className="icon-toggle-btn settings-btn"
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
          instanceNumber={getDisplayNumber(instance.id)}
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
