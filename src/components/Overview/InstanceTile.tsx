import { useProjectStore, useAppStore } from '@/stores'
import { Instance } from '@/types'
import './InstanceTile.css'

interface InstanceTileProps {
  instance: Instance
}

export function InstanceTile({ instance }: InstanceTileProps) {
  const project = useProjectStore(state => state.getProject(instance.projectId))
  const focusInstance = useAppStore(state => state.focusInstance)

  if (!project) return null

  const statusIcon = {
    starting: '◌',
    working: '●',
    waiting: '⏳',
    paused: '⏸',
    stopped: '○'
  }[instance.status]

  const duration = Math.floor(
    (Date.now() - new Date(instance.startedAt).getTime()) / 1000 / 60
  )

  const recentOutput = instance.terminalHistory.slice(-8).join('')

  return (
    <div
      className="instance-tile"
      style={{ borderColor: project.color }}
      onClick={() => focusInstance(instance.id)}
    >
      <div className="tile-header" style={{ borderBottomColor: project.color }}>
        <div className="tile-title">
          <span className="project-name">{project.displayName}</span>
          <span className="project-path">{project.path}</span>
        </div>
        <div className="tile-status">
          <span className="status-icon">{statusIcon}</span>
          <span className="status-text">{instance.status}</span>
        </div>
      </div>

      <div className="tile-terminal">
        <pre>{recentOutput || 'Starting...'}</pre>
      </div>

      {instance.pendingQuestion && (
        <div className="tile-question">
          <span className="question-badge">?</span>
          <span className="question-text">
            {instance.pendingQuestion.text.slice(0, 100)}...
          </span>
        </div>
      )}

      <div className="tile-stats">
        <span>{(instance.tokensUsed / 1000).toFixed(0)}k</span>
        <span className="stats-divider">│</span>
        <span>${instance.costEstimate.toFixed(2)}</span>
        <span className="stats-divider">│</span>
        <span>{duration} min</span>
      </div>
    </div>
  )
}
