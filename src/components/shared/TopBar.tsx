import { useInstanceStore, useAppStore } from '@/stores'
import './TopBar.css'

export function TopBar() {
  const instances = useInstanceStore(state => state.instances)
  const toggleSettings = useAppStore(state => state.toggleSettings)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)

  const totalTokens = instances.reduce((sum, i) => sum + i.tokensUsed, 0)
  const totalCost = instances.reduce((sum, i) => sum + i.costEstimate, 0)

  return (
    <div className="top-bar">
      <div className="top-bar-left">
        <span className="app-name">◉ Jenklaud</span>
      </div>

      <div className="top-bar-center">
        <span className="stat">{instances.length} instances</span>
        <span className="stat-divider">│</span>
        <span className="stat">{(totalTokens / 1000).toFixed(1)}k tokens</span>
        <span className="stat-divider">│</span>
        <span className="stat">${totalCost.toFixed(2)}</span>
      </div>

      <div className="top-bar-right">
        <button className="btn btn-primary" onClick={toggleNewInstanceModal}>
          + New
        </button>
        <button className="btn btn-icon" onClick={toggleSettings}>
          ⚙
        </button>
      </div>
    </div>
  )
}
