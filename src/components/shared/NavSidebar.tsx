import { useAppStore, useInstanceStore } from '@/stores'
import './NavSidebar.css'

export function NavSidebar() {
  const viewMode = useAppStore(state => state.viewMode)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const focusInstance = useAppStore(state => state.focusInstance)
  const toggleSettings = useAppStore(state => state.toggleSettings)
  const instances = useInstanceStore(state => state.instances)

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'working': return '#81C784'
      case 'waiting': return '#FFB74D'
      case 'paused': return '#90A4AE'
      default: return '#666'
    }
  }

  return (
    <nav className="nav-sidebar">
      <div className="nav-top">
        <button
          className={`nav-btn ${viewMode === 'overview' ? 'active' : ''}`}
          onClick={returnToOverview}
          title="Overview"
        >
          <span className="nav-icon">▣</span>
        </button>

        <div className="nav-divider" />

        {instances.map(instance => (
          <button
            key={instance.id}
            className={`nav-btn ${focusedInstanceId === instance.id ? 'active' : ''}`}
            onClick={() => focusInstance(instance.id)}
            title={`#${instance.instanceNumber} ${instance.title}`}
          >
            <span className="nav-instance-num">{instance.instanceNumber}</span>
            <span
              className="nav-status-dot"
              style={{ backgroundColor: getStatusColor(instance.status) }}
            />
          </button>
        ))}
      </div>

      <div className="nav-bottom">
        <button
          className="nav-btn"
          onClick={toggleSettings}
          title="Settings"
        >
          <span className="nav-icon">⚙</span>
        </button>
      </div>
    </nav>
  )
}
