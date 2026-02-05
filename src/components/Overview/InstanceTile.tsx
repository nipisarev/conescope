import { useState, useEffect, useRef } from 'react'
import { useProjectStore, useInstanceStore, useAppStore, useSettingsStore } from '@/stores'
import { Instance } from '@/types'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from '@xterm/addon-fit'
import { EditPencil } from 'iconoir-react'
import 'xterm/css/xterm.css'
import './InstanceTile.css'

interface InstanceTileProps {
  instance: Instance
}

export function InstanceTile({ instance }: InstanceTileProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editTitle, setEditTitle] = useState(instance.title)
  const terminalContainerRef = useRef<HTMLDivElement>(null)
  const terminalRef = useRef<XTerm | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)

  const project = useProjectStore(state => state.getProject(instance.projectId))
  const updateInstanceDb = useInstanceStore(state => state.updateInstanceDb)
  const getDisplayNumber = useInstanceStore(state => state.getDisplayNumber)
  const focusInstance = useAppStore(state => state.focusInstance)
  const terminalFontSize = useSettingsStore(state => state.terminalFontSize)

  // Initialize xterm.js
  useEffect(() => {
    if (!terminalContainerRef.current) return

    const terminal = new XTerm({
      theme: {
        background: '#252526',
        foreground: '#d4d4d4',
        cursor: '#252526', // Hide cursor in preview
        cursorAccent: '#252526',
      },
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      fontSize: terminalFontSize,
      lineHeight: 1.1,
      cursorBlink: false,
      scrollback: 1000,
      disableStdin: true, // Read-only preview
      allowProposedApi: true,
    })

    const fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)

    terminal.open(terminalContainerRef.current)

    // Delay fit() to allow terminal to fully initialize
    const fitTimeout = setTimeout(() => {
      try {
        fitAddon.fit()
        // Send dimensions to PTY so Claude Code adapts
        const { cols, rows } = terminal
        window.electronAPI.resizeInstance(instance.id, cols, rows)
      } catch (e) {
        // Ignore fit errors during initialization
      }
    }, 100)

    terminalRef.current = terminal
    fitAddonRef.current = fitAddon

    // Write existing history
    for (const chunk of instance.terminalHistory) {
      terminal.write(chunk)
    }

    // Listen for new output
    const cleanupOutput = window.electronAPI.onInstanceOutput((id, data) => {
      if (id === instance.id && terminalRef.current) {
        terminalRef.current.write(data)
      }
    })

    // Handle resize with debounce
    let resizeTimeout: NodeJS.Timeout
    const handleResize = () => {
      clearTimeout(resizeTimeout)
      resizeTimeout = setTimeout(() => {
        try {
          if (fitAddonRef.current && terminalRef.current) {
            fitAddonRef.current.fit()
            // Send new dimensions to PTY so Claude Code adapts
            const { cols, rows } = terminalRef.current
            window.electronAPI.resizeInstance(instance.id, cols, rows)
          }
        } catch (e) {
          // Ignore fit errors
        }
      }, 16)
    }

    const resizeObserver = new ResizeObserver(handleResize)
    resizeObserver.observe(terminalContainerRef.current)

    // Also listen for window resize
    window.addEventListener('resize', handleResize)

    return () => {
      clearTimeout(fitTimeout)
      clearTimeout(resizeTimeout)
      resizeObserver.disconnect()
      window.removeEventListener('resize', handleResize)
      cleanupOutput()
      terminal.dispose()
    }
  }, [instance.id])

  // React to font size changes
  useEffect(() => {
    if (terminalRef.current && fitAddonRef.current) {
      terminalRef.current.options.fontSize = terminalFontSize
      fitAddonRef.current.fit()
      const { cols, rows } = terminalRef.current
      window.electronAPI.resizeInstance(instance.id, cols, rows)
    }
  }, [terminalFontSize, instance.id])

  if (!project) return null

  const shortenPath = (fullPath: string) => {
    return fullPath.replace(/^\/Users\/[^/]+/, '~')
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
                #{getDisplayNumber(instance.id)}
              </span>
              <span className="tile-title" style={{ color: project.color }}>{instance.title}</span>
              <button
                className="tile-edit-btn"
                onClick={e => {
                  e.stopPropagation()
                  setIsEditing(true)
                }}
              >
                <EditPencil width={12} height={12} />
              </button>
              <span className="tile-status-dot" style={{ backgroundColor: statusColor }} title={instance.status} />
            </>
          )}
        </div>
        <div className="tile-meta">
          <span className="tile-path">{shortenPath(project.path)}</span>
          <span className="tile-tokens">{(instance.tokensUsed / 1000).toFixed(1)}k</span>
        </div>
      </div>

      <div className="tile-preview">
        <div ref={terminalContainerRef} className="tile-terminal-preview" />
      </div>
    </div>
  )
}
