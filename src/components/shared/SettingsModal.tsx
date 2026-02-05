import { useAppStore, useSettingsStore } from "@/stores";
import "./SettingsModal.css";

export function SettingsModal() {
  const toggleSettings = useAppStore((state) => state.toggleSettings);
  const theme = useSettingsStore((state) => state.theme);
  const editorFontSize = useSettingsStore((state) => state.editorFontSize);
  const terminalFontSize = useSettingsStore((state) => state.terminalFontSize);
  const setSetting = useSettingsStore((state) => state.setSetting);

  const handleClose = () => {
    toggleSettings();
  };

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div className="settings-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Settings</h2>
          <button className="close-btn" onClick={handleClose}>
            ×
          </button>
        </div>

        <div className="settings-content">
          <div className="settings-section">
            <h3>Appearance</h3>

            <div className="setting-row">
              <label>Theme</label>
              <select
                value={theme}
                onChange={(e) =>
                  setSetting("theme", e.target.value as "dark" | "light")
                }
              >
                <option value="dark">Dark</option>
                <option value="light">Light (coming soon)</option>
              </select>
            </div>
          </div>

          <div className="settings-section">
            <h3>Editor</h3>

            <div className="setting-row">
              <label>Font Size</label>
              <div className="setting-input-group">
                <input
                  type="number"
                  min="10"
                  max="24"
                  value={editorFontSize}
                  onChange={(e) =>
                    setSetting("editorFontSize", parseInt(e.target.value, 10))
                  }
                />
                <span className="input-suffix">px</span>
              </div>
            </div>
          </div>

          <div className="settings-section">
            <h3>Terminal</h3>

            <div className="setting-row">
              <label>Font Size</label>
              <div className="setting-input-group">
                <input
                  type="number"
                  min="10"
                  max="24"
                  value={terminalFontSize}
                  onChange={(e) =>
                    setSetting("terminalFontSize", parseInt(e.target.value, 10))
                  }
                />
                <span className="input-suffix">px</span>
              </div>
            </div>
          </div>

          <div className="settings-section">
            <h3>About</h3>
            <div className="about-info">
              <img
                src={
                  theme === "dark"
                    ? "/conescope-light.png"
                    : "/conescope-dark.png"
                }
                alt="Conescope"
                className="about-logo"
              />
              <p>
                <strong>Conescope</strong> v0.1.0
              </p>
              <p className="about-description">
                A command center for managing multiple AI Agents.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
