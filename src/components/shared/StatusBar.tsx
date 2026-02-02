import { useInstanceStore } from '@/stores'
import './StatusBar.css'

export function StatusBar() {
  const instances = useInstanceStore(state => state.instances)

  const totalTokens = instances.reduce((sum, i) => sum + i.tokensUsed, 0)
  const totalCost = instances.reduce((sum, i) => sum + i.costEstimate, 0)
  const workingCount = instances.filter(i => i.status === 'working').length
  const waitingCount = instances.filter(i => i.status === 'waiting').length

  return (
    <footer className="status-bar">
      <div className="status-item">
        <span className="status-label">{instances.length}</span>
        <span className="status-text">instances</span>
      </div>

      {workingCount > 0 && (
        <div className="status-item">
          <span className="status-dot working" />
          <span className="status-label">{workingCount}</span>
          <span className="status-text">working</span>
        </div>
      )}

      {waitingCount > 0 && (
        <div className="status-item">
          <span className="status-dot waiting" />
          <span className="status-label">{waitingCount}</span>
          <span className="status-text">waiting</span>
        </div>
      )}

      <div className="status-spacer" />

      <div className="status-item">
        <span className="status-label">{(totalTokens / 1000).toFixed(1)}k</span>
        <span className="status-text">tokens</span>
      </div>

      <div className="status-item">
        <span className="status-label">${totalCost.toFixed(2)}</span>
      </div>
    </footer>
  )
}
