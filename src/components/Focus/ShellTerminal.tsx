import { useEffect, useRef } from 'react'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from '@xterm/addon-fit'
import { useTerminalTabStore, useSettingsStore } from '@/stores'
import 'xterm/css/xterm.css'
import './Terminal.css'

interface ShellTerminalProps {
  tabId: string
}

export function ShellTerminal({ tabId }: ShellTerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const terminalRef = useRef<XTerm | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)
  const tab = useTerminalTabStore(state => state.tabs.find(t => t.id === tabId))
  const appendOutput = useTerminalTabStore(state => state.appendOutput)
  const terminalFontSize = useSettingsStore(state => state.terminalFontSize)

  useEffect(() => {
    if (!containerRef.current || !tab) return

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
      windowsMode: false,
      convertEol: false,
      scrollOnUserInput: true,
    })

    const fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)

    terminal.open(containerRef.current)

    const fitTimeout = setTimeout(() => {
      try {
        fitAddon.fit()
        const { cols, rows } = terminal
        window.electronAPI.resizeShellTerminal(tabId, cols, rows)
      } catch (e) {
        // Ignore fit errors
      }
    }, 100)

    terminal.onData((data) => {
      window.electronAPI.sendShellInput(tabId, data)
    })

    terminalRef.current = terminal
    fitAddonRef.current = fitAddon

    // Write existing history
    if (tab.history) {
      for (const chunk of tab.history) {
        terminal.write(chunk)
      }
    }

    // Listen for shell output
    const cleanupOutput = window.electronAPI.onShellOutput((id, data) => {
      if (id === tabId && terminalRef.current) {
        terminalRef.current.write(data)
        appendOutput(tabId, data)
      }
    })

    // Handle resize
    let resizeTimeout: NodeJS.Timeout
    const handleResize = () => {
      clearTimeout(resizeTimeout)
      resizeTimeout = setTimeout(() => {
        try {
          if (fitAddonRef.current && terminalRef.current) {
            fitAddonRef.current.fit()
            const { cols, rows } = terminalRef.current
            window.electronAPI.resizeShellTerminal(tabId, cols, rows)
          }
        } catch (e) {
          // Ignore
        }
      }, 16)
    }

    const resizeObserver = new ResizeObserver(handleResize)
    resizeObserver.observe(containerRef.current)
    window.addEventListener('resize', handleResize)

    return () => {
      clearTimeout(fitTimeout)
      clearTimeout(resizeTimeout)
      resizeObserver.disconnect()
      window.removeEventListener('resize', handleResize)
      cleanupOutput()
      terminal.dispose()
    }
  }, [tabId])

  // React to font size changes
  useEffect(() => {
    if (terminalRef.current && fitAddonRef.current) {
      terminalRef.current.options.fontSize = terminalFontSize
      fitAddonRef.current.fit()
      const { cols, rows } = terminalRef.current
      window.electronAPI.resizeShellTerminal(tabId, cols, rows)
    }
  }, [terminalFontSize, tabId])

  return <div ref={containerRef} className="terminal-container" />
}
