import { useState } from 'react'
import { useProjectStore, useInstanceStore, useAppStore } from '@/stores'
import './NewInstanceModal.css'

export function NewInstanceModal() {
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const projects = useProjectStore(state => state.projects)
  const addProject = useProjectStore(state => state.addProject)
  const createInstance = useInstanceStore(state => state.createInstance)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const focusInstance = useAppStore(state => state.focusInstance)

  const handleSelectDirectory = async () => {
    const path = await window.electronAPI.selectDirectory()
    if (path) {
      setSelectedPath(path)
    }
  }

  const handleCreateInstance = async (projectPath: string) => {
    const project = await addProject(projectPath)
    // Create instance in store first (gets ID and instance number from DB)
    const instance = await createInstance(project.id, project.displayName)
    // Start the PTY process with the same instance ID
    await window.electronAPI.createInstance(instance.id, projectPath)
    toggleNewInstanceModal()
    focusInstance(instance.id)
  }

  const handleClose = () => {
    toggleNewInstanceModal()
    setSelectedPath(null)
  }

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>New Instance</h2>
          <button className="close-btn" onClick={handleClose}>×</button>
        </div>

        <div className="modal-content">
          {projects.length > 0 && (
            <div className="section">
              <h3>Recent Projects</h3>
              <div className="project-list">
                {projects.map(project => (
                  <button
                    key={project.id}
                    className="project-item"
                    onClick={() => handleCreateInstance(project.path)}
                  >
                    <div
                      className="project-color"
                      style={{ backgroundColor: project.color }}
                    />
                    <div className="project-info">
                      <span className="project-name">{project.displayName}</span>
                      <span className="project-path">{project.path}</span>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          )}

          <div className="section">
            <h3>Browse</h3>
            <button className="browse-btn" onClick={handleSelectDirectory}>
              Select Directory...
            </button>
            {selectedPath && (
              <div className="selected-path">
                <span>{selectedPath}</span>
                <button
                  className="launch-btn"
                  onClick={() => handleCreateInstance(selectedPath)}
                >
                  Launch
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
