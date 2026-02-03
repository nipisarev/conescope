import { useState } from 'react'
import { useAppStore, useInstanceStore, useProjectStore, useSettingsStore } from '@/stores'
import { ViewGrid, Folder, PageEdit, TerminalTag } from 'iconoir-react'
import { InstancePopup } from './InstancePopup'
import './NavSidebar.css'

export function NavSidebar() {
  const viewMode = useAppStore(state => state.viewMode)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const focusInstance = useAppStore(state => state.focusInstance)
  const instances = useInstanceStore(state => state.instances)
  const getProject = useProjectStore(state => state.getProject)

  const sessionState = useSettingsStore(state => state.sessionState)
  const toggleFolderPanel = useSettingsStore(state => state.toggleFolderPanel)
  const toggleEditorPanel = useSettingsStore(state => state.toggleEditorPanel)
  const toggleTerminalPanel = useSettingsStore(state => state.toggleTerminalPanel)

  const [showPopup, setShowPopup] = useState(false)

  const isFocusMode = viewMode === 'focus'

  return (
    <nav className="activity-bar">
      {/* Overview button */}
      <div
        className="overview-btn-wrapper"
        onMouseEnter={() => isFocusMode && setShowPopup(true)}
        onMouseLeave={() => setShowPopup(false)}
      >
        <button
          className={`activity-btn ${!isFocusMode ? 'active' : ''}`}
          onClick={returnToOverview}
          title="Overview"
        >
          <ViewGrid width={18} height={18} />
        </button>
        {isFocusMode && showPopup && instances.length > 0 && (
          <InstancePopup onClose={() => setShowPopup(false)} />
        )}
      </div>

      {isFocusMode ? (
        /* Focus mode: panel toggles */
        <>
          <div className="activity-bar-divider" />
          <div className="activity-bar-section">
            <button
              className={`activity-btn panel-toggle ${sessionState.folderPanelVisible ? 'panel-active' : ''}`}
              onClick={toggleFolderPanel}
              title="Toggle Folder Panel"
            >
              <Folder width={18} height={18} />
            </button>
            <button
              className={`activity-btn panel-toggle ${sessionState.editorPanelVisible ? 'panel-active' : ''}`}
              onClick={toggleEditorPanel}
              title="Toggle Editor Panel"
            >
              <PageEdit width={18} height={18} />
            </button>
            <button
              className={`activity-btn panel-toggle ${sessionState.terminalPanelVisible ? 'panel-active' : ''}`}
              onClick={toggleTerminalPanel}
              title="Toggle Terminal Panel"
            >
              <TerminalTag width={18} height={18} />
            </button>
          </div>
        </>
      ) : (
        /* Overview mode: instance numbers */
        instances.length > 0 && (
          <div className="activity-bar-section">
            {[...instances]
              .sort((a, b) => a.instanceNumber - b.instanceNumber)
              .map(instance => {
                const project = getProject(instance.projectId)
                const color = project?.color || '#888'
                const isActive = focusedInstanceId === instance.id

                return (
                  <button
                    key={instance.id}
                    className={`activity-btn ${isActive ? 'active' : ''}`}
                    style={{ color: isActive ? '#fff' : color }}
                    onClick={() => focusInstance(instance.id)}
                    title={`#${instance.instanceNumber} ${instance.title}`}
                  >
                    <span className="activity-btn-num">{instance.instanceNumber}</span>
                  </button>
                )
              })}
          </div>
        )
      )}
    </nav>
  )
}
