import { useInstanceStore } from '@/stores'
import { InstanceTile } from './InstanceTile'
import { EmptySlot } from './EmptySlot'
import './OverviewGrid.css'

export function OverviewGrid() {
  const instances = useInstanceStore(state => state.instances)

  // Calculate grid layout
  const count = instances.length
  let cols = 1
  let rows = 1

  if (count === 0) {
    cols = 1
    rows = 1
  } else if (count === 1) {
    cols = 1
    rows = 1
  } else if (count === 2) {
    cols = 2
    rows = 1
  } else if (count <= 4) {
    cols = 2
    rows = 2
  } else if (count <= 6) {
    cols = 3
    rows = 2
  } else {
    cols = 3
    rows = Math.ceil(count / 3)
  }

  const totalSlots = cols * rows
  const emptySlots = Math.max(0, totalSlots - count)

  // Only show one empty slot for the [+] button
  const showEmptySlot = emptySlots > 0 || count === 0

  return (
    <div
      className="overview-grid"
      style={{
        gridTemplateColumns: `repeat(${cols}, 1fr)`,
        gridTemplateRows: `repeat(${rows}, 1fr)`
      }}
    >
      {[...instances].sort((a, b) => a.instanceNumber - b.instanceNumber).map(instance => (
        <InstanceTile key={instance.id} instance={instance} />
      ))}
      {showEmptySlot && <EmptySlot />}
    </div>
  )
}
