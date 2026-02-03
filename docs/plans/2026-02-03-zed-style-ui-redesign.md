# Zed-Style UI Redesign

## Overview

Redesign Jenklaud's navigation and controls to match Zed editor's minimal aesthetic: bottom activity bar, reduced borders, icon-only controls.

## Changes

### 1. Bottom Activity Bar

Replace left NavSidebar with horizontal bottom-left activity bar.

**Overview mode:**
```
[▣] [1] [2] [3]...
```
- Overview icon + instance numbers (colored by project, no borders)
- Active instance: accent background
- Size: ~24-28px icons, minimal padding

**Focus mode:**
```
[▣] | [📁] [📄] [⌨]
```
- Overview icon + thin divider + panel toggle icons
- Panel toggles: folder view, editor, terminal
- Active toggle: bold + accent color
- Inactive toggle: dim color
- Hover on overview icon: popup showing all instance numbers (colored, clickable)

### 2. Top Bar

**Overview mode (top-right):**
- `+ New` button
- `Questions` button
- `⚙` Settings icon (moved from left sidebar)

**Focus mode (top-right):**
- `−` Minimize icon (returns to overview)
- `×` Close icon (shows confirmation modal)
- Remove pause/resume functionality

**Focus mode (center):**
- Keep instance title with project color

### 3. Close Confirmation Modal

When clicking close:
- Show instance info: number + title
- Show status (running/idle)
- Warning text if actively running
- "Yes" / "No" buttons

### 4. Border Minimization

- All borders: 1px (down from current)
- Border color: `#2a2a2a` (subtler than `#3c3c3c`)
- Resize handles: 2px (down from 4px)
- Remove unnecessary dividers

### 5. Panel Visibility State

New state in settingsStore:
- `folderPanelVisible: boolean`
- `editorPanelVisible: boolean`
- `terminalPanelVisible: boolean`

Persist to session state.

## Files to Modify

1. `src/components/shared/NavSidebar.tsx` - Complete rewrite to bottom bar
2. `src/components/shared/NavSidebar.css` - New horizontal layout styles
3. `src/components/shared/TopBar.tsx` - Move settings, add minimize/close icons
4. `src/components/shared/TopBar.css` - Update styles
5. `src/components/Focus/FocusView.tsx` - Add panel visibility toggles
6. `src/components/Focus/FocusView.css` - Reduce borders
7. `src/stores/settingsStore.ts` - Add panel visibility state
8. `src/index.css` - Global border reduction

## New Components

1. `src/components/shared/CloseConfirmModal.tsx` - Confirmation dialog
2. `src/components/shared/CloseConfirmModal.css` - Modal styles
3. `src/components/shared/InstancePopup.tsx` - Hover popup for instances
4. `src/components/shared/InstancePopup.css` - Popup styles

## Implementation Order

1. Add panel visibility state to settingsStore
2. Create CloseConfirmModal component
3. Create InstancePopup component
4. Rewrite NavSidebar as bottom activity bar
5. Update TopBar for both modes
6. Update FocusView to use panel visibility
7. Reduce borders globally
