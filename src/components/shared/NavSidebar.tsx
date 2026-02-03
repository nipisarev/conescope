import { useState, useRef, useCallback } from 'react'
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
  const hideTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  const isFocusMode = viewMode === 'focus'

  // Token/cost calculations
  const totalTokens = instances.reduce((sum, i) => sum + i.tokensUsed, 0)
  const totalCost = instances.reduce((sum, i) => sum + i.costEstimate, 0)

  const handleMouseEnter = useCallback(() => {
    if (hideTimeoutRef.current) {
      clearTimeout(hideTimeoutRef.current)
      hideTimeoutRef.current = null
    }
    if (isFocusMode) {
      setShowPopup(true)
    }
  }, [isFocusMode])

  const handleMouseLeave = useCallback(() => {
    hideTimeoutRef.current = setTimeout(() => {
      setShowPopup(false)
    }, 300)
  }, [])

  return (
    <nav className="activity-bar">
      <div className="activity-bar-left">
        {/* Overview button */}
        <div
          className="overview-btn-wrapper"
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          <button
            className={`activity-btn ${!isFocusMode ? 'active' : ''}`}
            onClick={returnToOverview}
            title="Overview"
          >
            <ViewGrid width={14} height={14} />
          </button>
          {isFocusMode && showPopup && instances.length > 0 && (
            <InstancePopup
              onClose={() => setShowPopup(false)}
              onMouseEnter={handleMouseEnter}
              onMouseLeave={handleMouseLeave}
            />
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
                <Folder width={14} height={14} />
              </button>
              <button
                className={`activity-btn panel-toggle ${sessionState.editorPanelVisible ? 'panel-active' : ''}`}
                onClick={toggleEditorPanel}
                title="Toggle Editor Panel"
              >
                <PageEdit width={14} height={14} />
              </button>
              <button
                className={`activity-btn panel-toggle ${sessionState.terminalPanelVisible ? 'panel-active' : ''}`}
                onClick={toggleTerminalPanel}
                title="Toggle Terminal Panel"
              >
                <TerminalTag width={14} height={14} />
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
      </div>

      <div className="activity-bar-right">
        <span className="activity-stat">{(totalTokens / 1000).toFixed(1)}k tokens</span>
        <span className="activity-stat">${totalCost.toFixed(2)}</span>
      </div>
    </nav>
  )
}
