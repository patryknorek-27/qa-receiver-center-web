export function formatNumber(value: number | null, suffix = '') {
  if (value === null || Number.isNaN(value)) {
    return '--'
  }

  return `${Number.isInteger(value) ? value : value.toFixed(1)}${suffix}`
}

export function formatDuration(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return '--'
  }

  const totalSeconds = Math.max(0, Math.floor(value))
  const days = Math.floor(totalSeconds / 86400)
  const hours = Math.floor((totalSeconds % 86400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)

  if (days > 0) {
    return `${days}d ${hours}h`
  }

  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }

  return `${minutes}m`
}

export function formatBytes(value: number) {
  if (!Number.isFinite(value)) {
    return '--'
  }

  const units = ['B', 'KB', 'MB', 'GB']
  let size = value
  let unitIndex = 0
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex += 1
  }

  return `${size >= 10 ? size.toFixed(0) : size.toFixed(1)} ${units[unitIndex]}`
}

export function formatProgressPercent(value: number) {
  if (!Number.isFinite(value)) {
    return '--'
  }

  const safeValue = Math.max(0, Math.min(100, value))
  return `${safeValue >= 10 ? safeValue.toFixed(0) : safeValue.toFixed(1)}%`
}
