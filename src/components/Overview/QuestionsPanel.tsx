import { useMemo } from 'react'
import { useInstanceStore, useAppStore } from '@/stores'
import './QuestionsPanel.css'

export function QuestionsPanel() {
  const instances = useInstanceStore(state => state.instances)
  const getDisplayNumber = useInstanceStore(state => state.getDisplayNumber)
  const focusInstance = useAppStore(state => state.focusInstance)

  const waitingInstances = useMemo(() => {
    return instances.filter(i => i.status === 'waiting')
  }, [instances])

  const getTimeAgo = (date: string) => {
    const mins = Math.floor((Date.now() - new Date(date).getTime()) / 60000)
    if (mins < 1) return 'just now'
    if (mins === 1) return '1m ago'
    return `${mins}m ago`
  }

  return (
    <aside className="questions-panel">
      <div className="questions-header">
        QUESTIONS ({waitingInstances.length})
      </div>

      <div className="questions-list">
        {waitingInstances.length === 0 ? (
          <div className="questions-empty">No pending questions</div>
        ) : (
          waitingInstances.map(instance => (
            <div
              key={instance.id}
              className="question-item"
              onClick={() => focusInstance(instance.id)}
            >
              <div className="question-header">
                <span className="question-instance">#{getDisplayNumber(instance.id)}</span>
                <span className="question-title">{instance.title}</span>
              </div>
              <div className="question-preview">
                {instance.terminalHistory.slice(-1)[0] || 'Waiting for response...'}
              </div>
              <div className="question-time">{getTimeAgo(instance.startedAt)}</div>
            </div>
          ))
        )}
      </div>
    </aside>
  )
}
