import { useEffect, useRef } from 'react'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from '@xterm/addon-fit'
import { useInstanceStore, useSettingsStore } from '@/stores'
import 'xterm/css/xterm.css'
import './Terminal.css'

interface TerminalProps {
  instanceId: string
  onInput: (data: string) => void
}

export function Terminal({ instanceId, onInput }: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const terminalRef = useRef<XTerm | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)
  const instance = useInstanceStore(state => state.getInstance(instanceId))
  const terminalFontSize = useSettingsStore(state => state.terminalFontSize)
  const isShellInstance = instance?.type === 'terminal'

  useEffect(() => {
    if (!containerRef.current) return

    const terminal = new XTerm({
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
        cursorAccent: '#1e1e1e',
        selectionBackground: '#264f78'
      },
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      fontSize: terminalFontSize,
      lineHeight: 1.0,
      letterSpacing: 0,
      cursorBlink: true,
      scrollback: 10000,
      allowProposedApi: true,
      // Mouse support for Claude Code interactive UI
      windowsMode: false,
      convertEol: false,
      scrollOnUserInput: true,
    })

    const fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)

    terminal.open(containerRef.current)

    const resize = isShellInstance
      ? window.electronAPI.resizeShellTerminal
      : window.electronAPI.resizeInstance

    // Delay fit() to allow terminal to fully initialize
    const fitTimeout = setTimeout(() => {
      try {
        fitAddon.fit()
        // Send initial dimensions to PTY
        const { cols, rows } = terminal
        resize(instanceId, cols, rows)
      } catch (e) {
        // Ignore fit errors during initialization
      }
    }, 100)

    terminal.onData((data) => {
      onInput(data)
    })

    terminalRef.current = terminal
    fitAddonRef.current = fitAddon

    // Write existing terminal history
    if (instance?.terminalHistory) {
      for (const chunk of instance.terminalHistory) {
        terminal.write(chunk)
      }
    }

    // Listen for new output from this instance
    const onOutput = isShellInstance
      ? window.electronAPI.onShellOutput
      : window.electronAPI.onInstanceOutput
    const cleanupOutput = onOutput((id, data) => {
      if (id === instanceId && terminalRef.current) {
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
            // Send new dimensions to PTY
            const { cols, rows } = terminalRef.current
            resize(instanceId, cols, rows)
          }
        } catch (e) {
          // Ignore fit errors
        }
      }, 16) // ~60fps for smoother resizing
    }

    const resizeObserver = new ResizeObserver(handleResize)
    resizeObserver.observe(containerRef.current)

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
  }, [instanceId, onInput])

  // React to font size changes
  useEffect(() => {
    if (terminalRef.current && fitAddonRef.current) {
      terminalRef.current.options.fontSize = terminalFontSize
      fitAddonRef.current.fit()
      const { cols, rows } = terminalRef.current
      const resizeFn = isShellInstance
        ? window.electronAPI.resizeShellTerminal
        : window.electronAPI.resizeInstance
      resizeFn(instanceId, cols, rows)
    }
  }, [terminalFontSize, instanceId, isShellInstance])

  return <div ref={containerRef} className="terminal-container" />
}
