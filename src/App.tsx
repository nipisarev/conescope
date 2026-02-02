import { useAppStore } from '@/stores'
import { TopBar } from '@/components/shared/TopBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { InboxPanel } from '@/components/Overview/InboxPanel'
import { FocusView } from '@/components/Focus/FocusView'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)

  if (viewMode === 'focus') {
    return <FocusView />
  }

  return (
    <div className="app-container">
      <TopBar />
      <div className="main-content">
        <OverviewGrid />
        <InboxPanel />
      </div>
    </div>
  )
}
