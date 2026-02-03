import { useState, useEffect, useCallback, useRef } from 'react'
import { Tree, NodeRendererProps } from 'react-arborist'
import { useEditorStore } from '@/stores'
import './FileTree.css'

interface FileNode {
  id: string
  name: string
  path: string
  isDirectory: boolean
  children?: FileNode[]
}

interface FileTreeProps {
  projectPath: string
}

function FileTreeNode({ node, style, dragHandle }: NodeRendererProps<FileNode>) {
  const openFile = useEditorStore(state => state.openFile)

  const handleClick = () => {
    if (node.isLeaf) {
      openFile(node.data.path)
    } else {
      node.toggle()
    }
  }

  const getFileIcon = (name: string, isDirectory: boolean, isOpen: boolean) => {
    if (isDirectory) {
      return isOpen ? '📂' : '📁'
    }
    const ext = name.split('.').pop()?.toLowerCase()
    const icons: Record<string, string> = {
      ts: '🟦',
      tsx: '⚛️',
      js: '🟨',
      jsx: '⚛️',
      json: '📋',
      md: '📝',
      css: '🎨',
      html: '🌐',
      py: '🐍',
      gitignore: '🙈',
    }
    return icons[ext || ''] || '📄'
  }

  return (
    <div
      ref={dragHandle}
      style={style}
      className={`file-tree-node ${node.isSelected ? 'selected' : ''}`}
      onClick={handleClick}
    >
      <span className="file-tree-icon">
        {getFileIcon(node.data.name, node.data.isDirectory, node.isOpen)}
      </span>
      <span className="file-tree-name">{node.data.name}</span>
    </div>
  )
}

export function FileTree({ projectPath }: FileTreeProps) {
  const [data, setData] = useState<FileNode[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [dimensions, setDimensions] = useState({ width: 240, height: 500 })
  const containerRef = useRef<HTMLDivElement>(null)

  const loadDirectory = useCallback(async (dirPath: string): Promise<FileNode[]> => {
    const entries = await window.electronAPI.readDirectory(dirPath)

    // Filter out hidden files and node_modules
    const filtered = entries.filter(entry =>
      !entry.name.startsWith('.') &&
      entry.name !== 'node_modules' &&
      entry.name !== 'dist' &&
      entry.name !== 'dist-electron'
    )

    // Sort: directories first, then files, alphabetically
    const sorted = filtered.sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) {
        return a.isDirectory ? -1 : 1
      }
      return a.name.localeCompare(b.name)
    })

    const nodes: FileNode[] = []
    for (const entry of sorted) {
      const node: FileNode = {
        id: entry.path,
        name: entry.name,
        path: entry.path,
        isDirectory: entry.isDirectory,
      }

      if (entry.isDirectory) {
        node.children = await loadDirectory(entry.path)
      }

      nodes.push(node)
    }

    return nodes
  }, [])

  useEffect(() => {
    if (!projectPath) return

    setIsLoading(true)
    loadDirectory(projectPath)
      .then(setData)
      .finally(() => setIsLoading(false))
  }, [projectPath, loadDirectory])

  // Handle container resizing - re-run when data loads
  useEffect(() => {
    const container = containerRef.current
    if (!container || isLoading || data.length === 0) return

    const updateDimensions = () => {
      const rect = container.getBoundingClientRect()
      if (rect.height > 0 && rect.width > 0) {
        setDimensions({ width: rect.width, height: rect.height })
      }
    }

    // Initial measurement after layout settles
    const timeout = setTimeout(updateDimensions, 50)

    const resizeObserver = new ResizeObserver(updateDimensions)
    resizeObserver.observe(container)

    return () => {
      clearTimeout(timeout)
      resizeObserver.disconnect()
    }
  }, [isLoading, data.length])

  if (isLoading) {
    return (
      <div className="file-tree">
        <div className="file-tree-header">FILES</div>
        <div className="file-tree-content">
          <div className="file-tree-message">Loading files...</div>
        </div>
      </div>
    )
  }

  if (data.length === 0) {
    return (
      <div className="file-tree">
        <div className="file-tree-header">FILES</div>
        <div className="file-tree-content">
          <div className="file-tree-message">No files found</div>
        </div>
      </div>
    )
  }

  return (
    <div className="file-tree">
      <div className="file-tree-header">FILES</div>
      <div className="file-tree-content" ref={containerRef}>
        <Tree
          data={data}
          openByDefault={false}
          width={dimensions.width}
          height={dimensions.height}
          indent={16}
          rowHeight={28}
          overscanCount={5}
          disableDrag
          disableDrop
        >
          {FileTreeNode}
        </Tree>
      </div>
    </div>
  )
}
