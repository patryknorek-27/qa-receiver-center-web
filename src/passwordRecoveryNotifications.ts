export type PasswordRecoveryNotice = {
  ip: string
  jobId: string
  message: string
  receiverName?: string | null
  model?: string | null
}

export function recoveryNoticeKey(notice: Pick<PasswordRecoveryNotice, 'ip' | 'jobId'>) {
  return JSON.stringify([notice.ip, notice.jobId])
}

export function collectNewRecoveryNotices(notices: PasswordRecoveryNotice[], seen: Set<string>) {
  const fresh: PasswordRecoveryNotice[] = []
  for (const notice of notices) {
    const key = recoveryNoticeKey(notice)
    if (!seen.has(key)) {
      seen.add(key)
      fresh.push(notice)
    }
  }
  return fresh
}
