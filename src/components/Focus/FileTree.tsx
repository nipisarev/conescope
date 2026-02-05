import React, { useState, useEffect, useCallback, useRef } from 'react'
import { Tree, NodeRendererProps } from 'react-arborist'
import { useEditorStore } from '@/stores'
import {
  Folder,
  NavArrowRight,
  NavArrowDown,
  Code,
  CodeBrackets,
  Css3,
  Html5,
  Git,
  Npm,
  Terminal,
  Settings,
  Text,
  Page,
  MediaImage,
  MediaVideo,
  MusicDoubleNote,
  Database,
  Lock,
  Archive,
} from 'iconoir-react'
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
    const iconSize = 16
    const iconProps = { width: iconSize, height: iconSize }

    if (isDirectory) {
      return (
        <>
          {isOpen ? (
            <NavArrowDown {...iconProps} className="folder-arrow" />
          ) : (
            <NavArrowRight {...iconProps} className="folder-arrow" />
          )}
          <Folder {...iconProps} className="folder-icon" />
        </>
      )
    }

    const ext = name.split('.').pop()?.toLowerCase()
    const baseName = name.toLowerCase()

    // Special file names
    if (baseName === 'package.json' || baseName === 'package-lock.json') {
      return <Npm {...iconProps} style={{ color: '#cb3837' }} />
    }
    if (baseName === 'tsconfig.json' || baseName === 'jsconfig.json') {
      return <Settings {...iconProps} style={{ color: '#3178c6' }} />
    }
    if (baseName === '.gitignore' || baseName === '.gitattributes') {
      return <Git {...iconProps} style={{ color: '#f05032' }} />
    }
    if (baseName === 'dockerfile' || baseName.startsWith('docker-compose')) {
      return <Archive {...iconProps} style={{ color: '#2496ed' }} />
    }
    if (baseName === '.env' || baseName.startsWith('.env.')) {
      return <Lock {...iconProps} style={{ color: '#ecd53f' }} />
    }
    if (baseName === 'readme.md' || baseName === 'readme') {
      return <Text {...iconProps} style={{ color: '#519aba' }} />
    }

    // By extension
    const iconMap: Record<string, React.ReactNode> = {
      // TypeScript
      ts: <Code {...iconProps} style={{ color: '#3178c6' }} />,
      tsx: <Code {...iconProps} style={{ color: '#61dafb' }} />,
      // JavaScript
      js: <Code {...iconProps} style={{ color: '#f7df1e' }} />,
      jsx: <Code {...iconProps} style={{ color: '#61dafb' }} />,
      mjs: <Code {...iconProps} style={{ color: '#f7df1e' }} />,
      cjs: <Code {...iconProps} style={{ color: '#f7df1e' }} />,
      // Web
      html: <Html5 {...iconProps} style={{ color: '#e34f26' }} />,
      htm: <Html5 {...iconProps} style={{ color: '#e34f26' }} />,
      css: <Css3 {...iconProps} style={{ color: '#1572b6' }} />,
      scss: <Css3 {...iconProps} style={{ color: '#c6538c' }} />,
      sass: <Css3 {...iconProps} style={{ color: '#c6538c' }} />,
      less: <Css3 {...iconProps} style={{ color: '#1d365d' }} />,
      // Data
      json: <CodeBrackets {...iconProps} style={{ color: '#cbcb41' }} />,
      yaml: <CodeBrackets {...iconProps} style={{ color: '#cb171e' }} />,
      yml: <CodeBrackets {...iconProps} style={{ color: '#cb171e' }} />,
      toml: <CodeBrackets {...iconProps} style={{ color: '#9c4121' }} />,
      xml: <CodeBrackets {...iconProps} style={{ color: '#e37933' }} />,
      // Markdown/Text
      md: <Text {...iconProps} style={{ color: '#519aba' }} />,
      mdx: <Text {...iconProps} style={{ color: '#fcb32c' }} />,
      txt: <Text {...iconProps} style={{ color: '#89a3b1' }} />,
      // Python
      py: <Code {...iconProps} style={{ color: '#3776ab' }} />,
      pyw: <Code {...iconProps} style={{ color: '#3776ab' }} />,
      pyx: <Code {...iconProps} style={{ color: '#3776ab' }} />,
      // Ruby
      rb: <Code {...iconProps} style={{ color: '#cc342d' }} />,
      erb: <Code {...iconProps} style={{ color: '#cc342d' }} />,
      // PHP
      php: <Code {...iconProps} style={{ color: '#777bb4' }} />,
      // Go
      go: <Code {...iconProps} style={{ color: '#00add8' }} />,
      // Rust
      rs: <Code {...iconProps} style={{ color: '#dea584' }} />,
      // Java/Kotlin
      java: <Code {...iconProps} style={{ color: '#b07219' }} />,
      kt: <Code {...iconProps} style={{ color: '#a97bff' }} />,
      kts: <Code {...iconProps} style={{ color: '#a97bff' }} />,
      // C/C++/C#
      c: <Code {...iconProps} style={{ color: '#555555' }} />,
      h: <Code {...iconProps} style={{ color: '#555555' }} />,
      cpp: <Code {...iconProps} style={{ color: '#f34b7d' }} />,
      hpp: <Code {...iconProps} style={{ color: '#f34b7d' }} />,
      cc: <Code {...iconProps} style={{ color: '#f34b7d' }} />,
      cs: <Code {...iconProps} style={{ color: '#178600' }} />,
      // Swift
      swift: <Code {...iconProps} style={{ color: '#f05138' }} />,
      // Shell
      sh: <Terminal {...iconProps} style={{ color: '#89e051' }} />,
      bash: <Terminal {...iconProps} style={{ color: '#89e051' }} />,
      zsh: <Terminal {...iconProps} style={{ color: '#89e051' }} />,
      fish: <Terminal {...iconProps} style={{ color: '#89e051' }} />,
      // SQL
      sql: <Database {...iconProps} style={{ color: '#e38c00' }} />,
      // Config
      ini: <Settings {...iconProps} style={{ color: '#6d8086' }} />,
      conf: <Settings {...iconProps} style={{ color: '#6d8086' }} />,
      config: <Settings {...iconProps} style={{ color: '#6d8086' }} />,
      // Lock files
      lock: <Lock {...iconProps} style={{ color: '#9b9b9b' }} />,
      // Images
      png: <MediaImage {...iconProps} style={{ color: '#a074c4' }} />,
      jpg: <MediaImage {...iconProps} style={{ color: '#a074c4' }} />,
      jpeg: <MediaImage {...iconProps} style={{ color: '#a074c4' }} />,
      gif: <MediaImage {...iconProps} style={{ color: '#a074c4' }} />,
      svg: <MediaImage {...iconProps} style={{ color: '#ffb13b' }} />,
      webp: <MediaImage {...iconProps} style={{ color: '#a074c4' }} />,
      ico: <MediaImage {...iconProps} style={{ color: '#a074c4' }} />,
      // Video
      mp4: <MediaVideo {...iconProps} style={{ color: '#fd4c5c' }} />,
      webm: <MediaVideo {...iconProps} style={{ color: '#fd4c5c' }} />,
      mov: <MediaVideo {...iconProps} style={{ color: '#fd4c5c' }} />,
      avi: <MediaVideo {...iconProps} style={{ color: '#fd4c5c' }} />,
      // Audio
      mp3: <MusicDoubleNote {...iconProps} style={{ color: '#ec407a' }} />,
      wav: <MusicDoubleNote {...iconProps} style={{ color: '#ec407a' }} />,
      ogg: <MusicDoubleNote {...iconProps} style={{ color: '#ec407a' }} />,
      flac: <MusicDoubleNote {...iconProps} style={{ color: '#ec407a' }} />,
      // Archives
      zip: <Archive {...iconProps} style={{ color: '#afb42b' }} />,
      tar: <Archive {...iconProps} style={{ color: '#afb42b' }} />,
      gz: <Archive {...iconProps} style={{ color: '#afb42b' }} />,
      rar: <Archive {...iconProps} style={{ color: '#afb42b' }} />,
      '7z': <Archive {...iconProps} style={{ color: '#afb42b' }} />,
      // Git
      gitignore: <Git {...iconProps} style={{ color: '#f05032' }} />,
      gitattributes: <Git {...iconProps} style={{ color: '#f05032' }} />,
      gitmodules: <Git {...iconProps} style={{ color: '#f05032' }} />,
      // Vue/Svelte
      vue: <Code {...iconProps} style={{ color: '#42b883' }} />,
      svelte: <Code {...iconProps} style={{ color: '#ff3e00' }} />,
      // Elixir/Erlang
      ex: <Code {...iconProps} style={{ color: '#6e4a7e' }} />,
      exs: <Code {...iconProps} style={{ color: '#6e4a7e' }} />,
      erl: <Code {...iconProps} style={{ color: '#b83998' }} />,
      // Haskell
      hs: <Code {...iconProps} style={{ color: '#5e5086' }} />,
      // Lua
      lua: <Code {...iconProps} style={{ color: '#000080' }} />,
      // R
      r: <Code {...iconProps} style={{ color: '#276dc3' }} />,
      // Scala
      scala: <Code {...iconProps} style={{ color: '#c22d40' }} />,
      // Clojure
      clj: <Code {...iconProps} style={{ color: '#63b132' }} />,
      cljs: <Code {...iconProps} style={{ color: '#63b132' }} />,
      // Dart
      dart: <Code {...iconProps} style={{ color: '#00b4ab' }} />,
      // GraphQL
      graphql: <CodeBrackets {...iconProps} style={{ color: '#e10098' }} />,
      gql: <CodeBrackets {...iconProps} style={{ color: '#e10098' }} />,
      // Prisma
      prisma: <Database {...iconProps} style={{ color: '#2d3748' }} />,
      // Env
      env: <Lock {...iconProps} style={{ color: '#ecd53f' }} />,
    }

    return iconMap[ext || ''] || <Page {...iconProps} style={{ color: '#6d8086' }} />
  }

  return (
    <div
      ref={dragHandle}
      style={style}
      className={`file-tree-node ${node.isSelected ? 'selected' : ''} ${node.data.isDirectory ? 'is-directory' : ''}`}
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
        <div className="file-tree-content">
          <div className="file-tree-message">Loading files...</div>
        </div>
      </div>
    )
  }

  if (data.length === 0) {
    return (
      <div className="file-tree">
        <div className="file-tree-content">
          <div className="file-tree-message">No files found</div>
        </div>
      </div>
    )
  }

  return (
    <div className="file-tree">
      <div className="file-tree-content" ref={containerRef}>
        <Tree
          data={data}
          openByDefault={false}
          width={dimensions.width}
          height={dimensions.height}
          indent={16}
          rowHeight={28}
          paddingTop={4}
          paddingBottom={4}
          overscanCount={5}
          disableDrag
          disableDrop
          className="file-tree-arborist"
        >
          {FileTreeNode}
        </Tree>
      </div>
    </div>
  )
}
