import { useEffect, useId, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { AlertTriangle, Check, Eye, VolumeX, X } from 'lucide-react'
import { collectNewRecoveryNotices, recoveryNoticeKey, type PasswordRecoveryNotice } from './passwordRecoveryNotifications'

export default function PasswordRecoveryAlerts() {
  const [notices, setNotices] = useState<PasswordRecoveryNotice[]>([])
  const [error, setError] = useState('')
  const [soundError, setSoundError] = useState(false)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [acknowledged, setAcknowledged] = useState(new Set<string>())
  const seen = useRef(new Set<string>())
  const dialog = useRef<HTMLDialogElement>(null)
  const acknowledgeButton = useRef<HTMLButtonElement>(null)
  const titleId = useId()
  const descriptionId = useId()
  const showDialog = notices.length > 0 && (dialogOpen || notices.some((notice) => !acknowledged.has(recoveryNoticeKey(notice))))

  useEffect(() => {
    if (!isTauri()) return
    let cancelled = false
    let timer: number | undefined
    const refresh = async () => {
      try {
        const result = await invoke<PasswordRecoveryNotice[]>('get_password_recovery_notices')
        if (!cancelled) {
          setNotices(result)
          setError('')
          const fresh = collectNewRecoveryNotices(result, seen.current)
          if (fresh.length > 0) {
            setDialogOpen(true)
            setSoundError(false)
            // Only new incidents request a sound. Polling and acknowledgement never retry it.
            void invoke('play_password_recovery_sound', {
              noticeIds: fresh.map(({ ip, jobId }) => ({ ip, jobId })),
            }).catch(() => { if (!cancelled) setSoundError(true) })
          }
          if (result.length === 0) setDialogOpen(false)
        }
      } catch {
        if (!cancelled) setError('Password recovery records could not be checked. Do not start another firmware update until this is resolved.')
      }
      if (!cancelled) timer = window.setTimeout(() => void refresh(), 2000)
    }
    void refresh()
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [])

  useEffect(() => {
    if (!showDialog) return
    const element = dialog.current
    if (!element) return
    const previousFocus = document.activeElement
    element.showModal()
    acknowledgeButton.current?.focus()
    return () => {
      element.close()
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus()
    }
  }, [showDialog])

  // Acknowledgement is UI-only. It neither clears the recovery record nor calls a receiver.
  const acknowledge = () => {
    setAcknowledged((current) => new Set([...current, ...notices.map(recoveryNoticeKey)]))
    setDialogOpen(false)
  }

  if (!error && notices.length === 0) return null
  return (
    <>
      <section className="password-recovery-alert" role="alert" aria-live="assertive">
        <AlertTriangle size={22} aria-hidden="true" />
        <div className="password-recovery-alert-copy">
          <strong>Password recovery requires attention</strong>
          {error && <p>{error}</p>}
          {notices.length > 0 && <p>{notices.map((notice) => notice.ip).join(', ')}</p>}
        </div>
        {notices.length > 0 && (
          <button className="secondary-button" type="button" onClick={() => setDialogOpen(true)}>
            <Eye size={16} aria-hidden="true" /> View details
          </button>
        )}
      </section>
      {showDialog && createPortal(
        <dialog
          ref={dialog}
          className="password-recovery-dialog"
          role="alertdialog"
          aria-modal="true"
          aria-labelledby={titleId}
          aria-describedby={descriptionId}
          onCancel={(event) => { event.preventDefault(); acknowledge() }}
        >
          <header className="password-recovery-dialog-header">
            <AlertTriangle size={26} aria-hidden="true" />
            <div>
              <h2 id={titleId}>Password recovery requires attention</h2>
              <p>{notices.length} {notices.length === 1 ? 'receiver' : 'receivers'}</p>
            </div>
            <button className="modal-close" type="button" aria-label="Acknowledge warning" title="Acknowledge warning" onClick={acknowledge}>
              <X size={18} aria-hidden="true" />
            </button>
          </header>
          <div className="password-recovery-dialog-body">
            <p id={descriptionId}>The password from qa-receivers.json could not be confirmed. Check the affected receivers before using other scripts.</p>
            {error && <p className="password-recovery-record-error">{error}</p>}
            <ul className="password-recovery-receivers" aria-label="Receivers requiring password recovery">
              {notices.map((notice) => (
                <li key={recoveryNoticeKey(notice)}>
                  <div className="password-recovery-receiver-heading">
                    <strong>{notice.ip}</strong>
                    {(notice.receiverName || notice.model) && <span>{[notice.receiverName, notice.model].filter(Boolean).join(' / ')}</span>}
                  </div>
                  <p>{notice.message}</p>
                </li>
              ))}
            </ul>
          </div>
          <footer className="password-recovery-dialog-footer">
            {soundError && <span className="password-recovery-sound-error"><VolumeX size={16} aria-hidden="true" /> Warning sound is unavailable.</span>}
            <button ref={acknowledgeButton} autoFocus className="primary-button" type="button" onClick={acknowledge}>
              <Check size={17} aria-hidden="true" /> Understood
            </button>
          </footer>
        </dialog>, document.body,
      )}
    </>
  )
}
