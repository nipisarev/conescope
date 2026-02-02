import { useAppStore, useSettingsStore } from '@/stores'
import './TopBar.css'

export function TopBar() {
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const toggleQuestionsPanel = useSettingsStore(state => state.toggleQuestionsPanel)
  const questionsPanelVisible = useSettingsStore(state => state.questionsPanelVisible)

  return (
    <header className="top-bar">
      <div className="top-bar-drag-region" />

      <div className="top-bar-left">
        <span className="app-title">◉ Jenklaud</span>
      </div>

      <div className="top-bar-right">
        <button className="top-bar-btn primary" onClick={toggleNewInstanceModal}>
          + New
        </button>
        <button
          className={`top-bar-btn ${questionsPanelVisible ? 'active' : ''}`}
          onClick={toggleQuestionsPanel}
        >
          Questions
        </button>
      </div>
    </header>
  )
}
