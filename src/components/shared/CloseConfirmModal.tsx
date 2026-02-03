import './CloseConfirmModal.css'

interface CloseConfirmModalProps {
  instanceNumber: number
  instanceTitle: string
  projectColor: string
  status: string
  onConfirm: () => void
  onCancel: () => void
}

export function CloseConfirmModal({
  instanceNumber,
  instanceTitle,
  projectColor,
  status,
  onConfirm,
  onCancel,
}: CloseConfirmModalProps) {
  const isRunning = status === 'running'

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="close-confirm-modal" onClick={e => e.stopPropagation()}>
        <div className="close-confirm-header">
          <h3>Close Instance?</h3>
        </div>

        <div className="close-confirm-content">
          <div className="close-confirm-info">
            <span className="close-confirm-number" style={{ color: projectColor }}>
              #{instanceNumber}
            </span>
            <span className="close-confirm-title">{instanceTitle}</span>
          </div>

          <div className={`close-confirm-status ${isRunning ? 'running' : ''}`}>
            <span>{isRunning ? '●' : '○'}</span>
            <span>{isRunning ? 'Running' : 'Idle'}</span>
          </div>

          {isRunning && (
            <div className="close-confirm-warning">
              This instance is actively running. Closing it will terminate the process.
            </div>
          )}

          <div className="close-confirm-actions">
            <button className="close-confirm-btn secondary" onClick={onCancel}>
              No
            </button>
            <button className="close-confirm-btn danger" onClick={onConfirm}>
              Yes
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
