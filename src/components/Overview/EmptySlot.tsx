import { useAppStore } from '@/stores'
import './OverviewGrid.css'

export function EmptySlot() {
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)

  return (
    <div className="empty-slot" onClick={toggleNewInstanceModal}>
      <div className="empty-slot-content">
        <span className="empty-slot-icon">+</span>
        <span className="empty-slot-text">New Instance</span>
      </div>
    </div>
  )
}
