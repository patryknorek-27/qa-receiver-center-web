export type FirmwareReleaseMode = 'Alpha' | 'Beta'

export function releaseMatchesMode(name: string, mode: FirmwareReleaseMode): boolean {
  const alpha = /\balpha\b/i.test(name)
  const beta = /\bbeta\b/i.test(name)

  return (!alpha && !beta) || (mode === 'Alpha' ? alpha : beta)
}
