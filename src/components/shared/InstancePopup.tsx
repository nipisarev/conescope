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

  return (
    <div className="instance-popup" onMouseEnter={onMouseEnter} onMouseLeave={onMouseLeave}>
      {[...instances]
        .sort((a, b) => a.instanceNumber - b.instanceNumber)
        .map(instance => {
          const project = getProject(instance.projectId)
          const color = project?.color || '#888'
          const isActive = focusedInstanceId === instance.id

          return (
            <button
              key={instance.id}
              className={`instance-popup-btn ${isActive ? 'active' : ''}`}
              style={{ color: isActive ? '#fff' : color }}
              onClick={() => handleClick(instance.id)}
              title={`#${instance.instanceNumber} ${instance.title}`}
            >
              {instance.instanceNumber}
            </button>
          )
        })}
    </div>
  )
}
