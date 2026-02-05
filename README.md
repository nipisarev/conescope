# Conescope

macOS Electron app for managing multiple Claude Code CLI instances in a unified interface.

## Features

- **Overview Grid**: Monitor multiple Claude Code sessions simultaneously with live terminal previews
- **Focus View**: Full terminal + file editor for individual instances
- **Project Management**: Organize instances by project with custom colors
- **Question Tracking**: Surface pending questions across all instances
- **SQLite Persistence**: Sessions and settings survive restarts

## Tech Stack

- **Frontend**: React 19, Zustand, xterm.js, CodeMirror
- **Backend**: Electron, node-pty, better-sqlite3
- **Build**: Vite, TypeScript, electron-builder

## Development

```bash
# Install dependencies
npm install

# Start dev server + Electron
npm run dev

# Type check
npx tsc --noEmit

# Build for production
npm run build && npm run build:electron
```

## Requirements

- macOS
- Node.js 18+
- [Claude Code CLI](https://github.com/anthropics/claude-code) installed

## License

ISC
