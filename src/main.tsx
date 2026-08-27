import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { ModusWcThemeProvider } from '@trimble-oss/moduswebcomponents-react'
import '@trimble-oss/moduswebcomponents-react/modus-wc-styles.css'
import './index.css'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ModusWcThemeProvider initialTheme={{ theme: 'modus-modern' }}>
      <App />
    </ModusWcThemeProvider>
  </StrictMode>,
)
