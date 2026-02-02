import { useEffect } from 'react'
import { useAppStore, useInstanceStore } from '@/stores'

export function useKeyboardShortcuts() {
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const focusInstance = useAppStore(state => state.focusInstance)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const toggleSettings = useAppStore(state => state.toggleSettings)
  const instances = useInstanceStore(state => state.instances)
  const getInstanceByNumber = useInstanceStore(state => state.getInstanceByNumber)

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check for Cmd (Mac) or Ctrl (Windows/Linux)
      const isMod = e.metaKey || e.ctrlKey

      if (!isMod) return

      // Cmd+0: Return to overview
      if (e.key === '0') {
        e.preventDefault()
        returnToOverview()
        return
      }

      // Cmd+1-9: Switch to instance by number
      if (e.key >= '1' && e.key <= '9') {
        e.preventDefault()
        const num = parseInt(e.key, 10)
        const instance = getInstanceByNumber(num)
        if (instance) {
          focusInstance(instance.id)
        }
        return
      }

      // Cmd+N: Open new instance modal
      if (e.key === 'n' || e.key === 'N') {
        e.preventDefault()
        toggleNewInstanceModal()
        return
      }

      // Cmd+,: Open settings
      if (e.key === ',') {
        e.preventDefault()
        toggleSettings()
        return
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [
    returnToOverview,
    focusInstance,
    toggleNewInstanceModal,
    toggleSettings,
    instances,
    getInstanceByNumber,
  ])
}
