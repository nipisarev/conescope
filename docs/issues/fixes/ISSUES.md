- In the terminal session, during selection didn't work standart behavior when you selected near to top or bottom, strarting scrolling, IDK how it's work, but u can reference to ZED or some different terminal realisations
- Need a context menu when I click on selection (right mouse button or double finger touch bar)
------------------
New Terminal Cmd+N
------------------
Copy
Paste
Select All
Clear
------------------
Close Terminal
------------------

- Currently when sidebar showing on top of the windows it closed after delay even if cursor in that moment placed into the sidebar window scope, but it shouldn't close in that situation, only when cursor left scope of sidebar window
- Blinking cursor in editor again became a black, IDK why...
- In editor we should store not saved yet changes for each file, currently when you move to another tab and return we lost last changes in the tab
- We should support autosave functions, each small period and when you change focus and etc.
- We lost behavior when we saving all configuration of focus mode configruation of windows (for each instance) - closed or not editor, filetree and terminal, size of window, fully broken
adasdsa


2026-02-15T14:09:32.500454Z ERROR gpui::platform::mac::metal_renderer: failed to render: scene too large: 0 paths, 0 shadows, 10993 quads, 0 underlines, 3084 mono, 0 poly, 0 surfaces. retrying with larger instance buffer size