import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { ModusWcThemeProvider } from '@trimble-oss/moduswebcomponents-react'
import '@trimble-oss/moduswebcomponents-react/modus-wc-styles.css'
import './index.css'
import App from './App.tsx'

const rootElement = document.getElementById('root')!
const startupSplash = document.getElementById('startup-splash')
const startupVideo = startupSplash?.querySelector('video')
let applicationReady = false
let startupVideoFinished = startupVideo?.ended ?? true

const closeStartupSplash = () => {
  if (!applicationReady || !startupVideoFinished) return
  window.requestAnimationFrame(() => {
    window.requestAnimationFrame(() => {
      startupSplash?.classList.add('is-hidden')
      window.setTimeout(() => startupSplash?.remove(), 250)
    })
  })
}

startupVideo?.addEventListener('ended', () => {
  startupVideoFinished = true
  closeStartupSplash()
}, { once: true })

startupVideo?.addEventListener('error', () => {
  startupVideoFinished = true
  closeStartupSplash()
}, { once: true })

window.addEventListener('qa-app-ready', () => {
  applicationReady = true
  closeStartupSplash()
}, { once: true })

createRoot(rootElement).render(
  <StrictMode>
    <ModusWcThemeProvider initialTheme={{ theme: 'modus-modern' }}>
      <App />
    </ModusWcThemeProvider>
  </StrictMode>,
)
