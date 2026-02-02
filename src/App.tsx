import { useAppStore } from '@/stores'
import { TopBar } from '@/components/shared/TopBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { InboxPanel } from '@/components/Overview/InboxPanel'
import { FocusView } from '@/components/Focus/FocusView'
import { NewInstanceModal } from '@/components/shared/NewInstanceModal'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)
  const newInstanceModalOpen = useAppStore(state => state.newInstanceModalOpen)

  if (viewMode === 'focus') {
    return (
      <>
        <FocusView />
        {newInstanceModalOpen && <NewInstanceModal />}
      </>
    )
  }

  return (
    <div className="app-container">
      <TopBar />
      <div className="main-content">
        <OverviewGrid />
        <InboxPanel />
      </div>
      {newInstanceModalOpen && <NewInstanceModal />}
    </div>
  )
}
