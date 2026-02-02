import { useMemo } from 'react'
import { useInstanceStore, useProjectStore, useAppStore } from '@/stores'
import { InboxItem, Urgency } from '@/types'
import './QuestionsPanel.css'

function calculateUrgency(askedAt: string): Urgency {
  const waitMs = Date.now() - new Date(askedAt).getTime()
  const waitMinutes = waitMs / 1000 / 60
  if (waitMinutes > 10) return 'urgent'
  if (waitMinutes > 5) return 'elevated'
  return 'normal'
}

interface QuestionItemRowProps {
  item: InboxItem
}

function QuestionItemRow({ item }: QuestionItemRowProps) {
  const project = useProjectStore(state => state.getProject(item.projectId))
  const focusInstance = useAppStore(state => state.focusInstance)

  if (!project) return null

  const waitMinutes = Math.floor(
    (Date.now() - new Date(item.askedAt).getTime()) / 1000 / 60
  )

  const urgencyClass = {
    normal: '',
    elevated: 'urgency-elevated',
    urgent: 'urgency-urgent'
  }[item.urgency]

  return (
    <div
      className={`questions-item ${urgencyClass}`}
      onClick={() => focusInstance(item.instanceId)}
    >
      <div
        className="questions-item-color"
        style={{ backgroundColor: project.color }}
      />
      <div className="questions-item-content">
        <span className="questions-item-project">{project.displayName}</span>
        <span className="questions-item-question">{item.question}</span>
        <span className="questions-item-time">{waitMinutes}m ago</span>
      </div>
      <div className="questions-item-actions">
        <button className="action-btn" title="Approve">✓</button>
        <button className="action-btn" title="Reject">✗</button>
        <button className="action-btn" title="Snooze">⏸</button>
      </div>
    </div>
  )
}

export function QuestionsPanel() {
  const instances = useInstanceStore(state => state.instances)
  const toggleQuestionsQueue = useAppStore(state => state.toggleQuestionsQueue)

  // Compute inbox items from instances - using useMemo to avoid infinite loops
  const inboxItems = useMemo((): InboxItem[] => {
    return instances
      .filter(i => i.pendingQuestion)
      .map(i => ({
        instanceId: i.id,
        projectId: i.projectId,
        question: i.pendingQuestion!.text,
        askedAt: i.pendingQuestion!.askedAt,
        urgency: calculateUrgency(i.pendingQuestion!.askedAt),
        snoozed: false
      }))
      .sort((a, b) => new Date(a.askedAt).getTime() - new Date(b.askedAt).getTime())
  }, [instances])

  return (
    <div className="questions-panel">
      <div className="questions-header">
        <span className="questions-title">QUESTIONS ({inboxItems.length})</span>
        <button className="expand-btn" onClick={toggleQuestionsQueue}>↗</button>
      </div>

      <div className="questions-list">
        {inboxItems.length === 0 ? (
          <div className="questions-empty">No pending questions</div>
        ) : (
          inboxItems.map(item => (
            <QuestionItemRow key={item.instanceId} item={item} />
          ))
        )}
      </div>

      {inboxItems.length > 0 && (
        <button className="view-queue-btn" onClick={toggleQuestionsQueue}>
          View Queue →
        </button>
      )}
    </div>
  )
}
