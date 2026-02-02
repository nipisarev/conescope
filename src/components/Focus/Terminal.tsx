import { useEffect, useRef } from 'react'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from '@xterm/addon-fit'
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
      fontFamily: '"SF Mono", Monaco, "Cascadia Code", monospace',
      fontSize: 13,
      lineHeight: 1.2,
      cursorBlink: true
    })

    const fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)

    terminal.open(containerRef.current)
    fitAddon.fit()

    terminal.onData((data) => {
      onInput(data)
    })

    terminalRef.current = terminal
    fitAddonRef.current = fitAddon

    // Listen for output from this instance
    window.electronAPI.onInstanceOutput((id, data) => {
      if (id === instanceId && terminalRef.current) {
        terminalRef.current.write(data)
      }
    })

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      fitAddonRef.current?.fit()
    })
    resizeObserver.observe(containerRef.current)

    return () => {
      resizeObserver.disconnect()
      terminal.dispose()
    }
  }, [instanceId, onInput])

  return <div ref={containerRef} className="terminal-container" />
}
