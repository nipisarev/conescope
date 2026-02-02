import { useInstanceStore, useProjectStore, useAppStore } from '@/stores'
import { InboxItem } from '@/types'
import './InboxPanel.css'

interface InboxItemRowProps {
  item: InboxItem
}

function InboxItemRow({ item }: InboxItemRowProps) {
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
      className={`inbox-item ${urgencyClass}`}
      onClick={() => focusInstance(item.instanceId)}
    >
      <div
        className="inbox-item-color"
        style={{ backgroundColor: project.color }}
      />
      <div className="inbox-item-content">
        <span className="inbox-item-project">{project.displayName}</span>
        <span className="inbox-item-question">{item.question}</span>
        <span className="inbox-item-time">⏱ {waitMinutes}m ago</span>
      </div>
      <div className="inbox-item-actions">
        <button className="action-btn" title="Approve">✓</button>
        <button className="action-btn" title="Reject">✗</button>
        <button className="action-btn" title="Snooze">⏸</button>
      </div>
    </div>
  )
}

export function InboxPanel() {
  const inboxItems = useInstanceStore(state => state.getInboxItems())
  const toggleQuestionsQueue = useAppStore(state => state.toggleQuestionsQueue)

  return (
    <div className="inbox-panel">
      <div className="inbox-header">
        <span className="inbox-title">INBOX ({inboxItems.length})</span>
        <button className="expand-btn" onClick={toggleQuestionsQueue}>↗</button>
      </div>

      <div className="inbox-list">
        {inboxItems.length === 0 ? (
          <div className="inbox-empty">No pending questions</div>
        ) : (
          inboxItems.map(item => (
            <InboxItemRow key={item.instanceId} item={item} />
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
