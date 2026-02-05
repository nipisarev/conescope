import { useInstanceStore, useProjectStore, useAppStore } from '@/stores'
import './InstancePopup.css'

interface InstancePopupProps {
  onClose: () => void
  onMouseEnter?: () => void
  onMouseLeave?: () => void
}

export function InstancePopup({ onClose, onMouseEnter, onMouseLeave }: InstancePopupProps) {
  const instances = useInstanceStore(state => state.instances)
  const getProject = useProjectStore(state => state.getProject)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const focusInstance = useAppStore(state => state.focusInstance)

  const handleClick = (instanceId: string) => {
    focusInstance(instanceId)
    onClose()
  }

  // Sort by original instance number for consistent ordering, then use index as display number
  const sortedInstances = [...instances].sort((a, b) => a.instanceNumber - b.instanceNumber)

  return (
    <div className="instance-popup" onMouseEnter={onMouseEnter} onMouseLeave={onMouseLeave}>
      {sortedInstances.map((instance, index) => {
        const project = instance.projectId ? getProject(instance.projectId) : null
        const color = instance.type === 'terminal'
          ? instance.color || '#888'
          : (project?.color || '#888')

        const shortenPath = (fullPath: string) => {
          return fullPath.replace(/^\/Users\/[^/]+/, '~')
        }
        const pathStr = project ? shortenPath(project.path) : '~'
        const isActive = focusedInstanceId === instance.id
        const displayNumber = index + 1

        return (
          <button
            key={instance.id}
            className={`instance-popup-btn ${isActive ? 'active' : ''}`}
            onClick={() => handleClick(instance.id)}
          >
            <span className="popup-number" style={{ color }}>{displayNumber}</span>
            <div className="popup-info">
              <span className="popup-title">{instance.title}</span>
              <span className="popup-path">{pathStr}</span>
            </div>
          </button>
        )
      })}
    </div>
  )
}
