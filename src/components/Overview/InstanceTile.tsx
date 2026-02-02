import { useState } from 'react'
import { useProjectStore, useInstanceStore, useAppStore } from '@/stores'
import { Instance } from '@/types'
import './InstanceTile.css'

interface InstanceTileProps {
  instance: Instance
}

export function InstanceTile({ instance }: InstanceTileProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editTitle, setEditTitle] = useState(instance.title)

  const project = useProjectStore(state => state.getProject(instance.projectId))
  const updateInstanceDb = useInstanceStore(state => state.updateInstanceDb)
  const focusInstance = useAppStore(state => state.focusInstance)

  if (!project) return null

  const shortenPath = (fullPath: string) => {
    const home = process.env.HOME || ''
    return fullPath.replace(home, '~')
  }

  const handleSaveTitle = async () => {
    if (editTitle.trim()) {
      await updateInstanceDb(instance.id, { title: editTitle.trim() })
    }
    setIsEditing(false)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSaveTitle()
    if (e.key === 'Escape') {
      setEditTitle(instance.title)
      setIsEditing(false)
    }
  }

  const lastOutput = instance.terminalHistory.slice(-5).join('')

  const statusColor = {
    working: '#81C784',
    waiting: '#FFB74D',
    paused: '#90A4AE',
    starting: '#64B5F6',
    stopped: '#666',
  }[instance.status] || '#666'

  return (
    <div className="instance-tile" onClick={() => focusInstance(instance.id)}>
      <div className="tile-header">
        <div className="tile-color" style={{ backgroundColor: project.color }} />
        <div className="tile-title-row">
          {isEditing ? (
            <input
              className="tile-title-input"
              value={editTitle}
              onChange={e => setEditTitle(e.target.value)}
              onBlur={handleSaveTitle}
              onKeyDown={handleKeyDown}
              onClick={e => e.stopPropagation()}
              autoFocus
            />
          ) : (
            <>
              <span className="tile-number" style={{ color: project.color }}>
                #{instance.instanceNumber}
              </span>
              <span className="tile-title">{instance.title}</span>
              <button
                className="tile-edit-btn"
                onClick={e => {
                  e.stopPropagation()
                  setIsEditing(true)
                }}
              >
                ✎
              </button>
            </>
          )}
        </div>
        <div className="tile-path">{shortenPath(project.path)}</div>
      </div>

      <div className="tile-preview">
        <pre className="tile-terminal">{lastOutput || '$ claude\n> Starting...'}</pre>
      </div>

      <div className="tile-footer">
        <span className="tile-status">
          <span className="tile-status-dot" style={{ backgroundColor: statusColor }} />
          {instance.status}
        </span>
        <span className="tile-tokens">{(instance.tokensUsed / 1000).toFixed(1)}k</span>
      </div>
    </div>
  )
}
