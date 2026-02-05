import { useMemo } from 'react'
import { useTerminalTabStore, useProjectStore, useInstanceStore } from '@/stores'
import { Plus, Xmark, TerminalTag } from 'iconoir-react'
import './TerminalTabs.css'

interface TerminalTabsProps {
  instanceId: string
}

export function TerminalTabs({ instanceId }: TerminalTabsProps) {
  const allTabs = useTerminalTabStore(state => state.tabs)
  const activeTabId = useTerminalTabStore(state => state.activeTabId[instanceId])
  const setActiveTab = useTerminalTabStore(state => state.setActiveTab)
  const addShellTab = useTerminalTabStore(state => state.addShellTab)
  const removeTab = useTerminalTabStore(state => state.removeTab)

  const instance = useInstanceStore(state => state.getInstance(instanceId))
  const project = useProjectStore(state => instance?.projectId ? state.getProject(instance.projectId) : undefined)

  const tabs = useMemo(() => allTabs.filter(t => t.instanceId === instanceId), [allTabs, instanceId])
  const activeTab = useMemo(() => tabs.find(t => t.id === activeTabId), [tabs, activeTabId])

  const handleAddTab = async () => {
    if (project) {
      await addShellTab(instanceId, project.path)
    }
  }

  const handleCloseTab = (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation()
    removeTab(tabId)
  }

  return (
    <div className="terminal-tabs">
      {tabs.map(tab => (
        <button
          key={tab.id}
          className={`terminal-tab ${activeTab?.id === tab.id ? 'active' : ''}`}
          onClick={() => setActiveTab(instanceId, tab.id)}
        >
          <TerminalTag width={14} height={14} />
          <span className="terminal-tab-title">{tab.title}</span>
          {tab.type === 'shell' && (
            <button
              className="terminal-tab-close"
              onClick={(e) => handleCloseTab(e, tab.id)}
            >
              <Xmark width={12} height={12} />
            </button>
          )}
        </button>
      ))}
      <button className="terminal-tab-add" onClick={handleAddTab} title="New terminal">
        <Plus width={14} height={14} />
      </button>
    </div>
  )
}
