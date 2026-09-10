export type FirmwareSourceMode = 'release' | 'local'

export function getGroupUpdateBlocker(state: {
  isGroupUpdate: boolean
  selectedCount: number
  hasActiveJob: boolean
  isStarting: boolean
  sourceMode: 'catalog' | 'local'
  releaseSelected: boolean
  buildSelected: boolean
  packagesReady: boolean
  isPackageCheckRunning: boolean
}) {
  if (!state.isGroupUpdate) return 'Open an Alpha or Beta receiver group before starting an update.'
  if (state.hasActiveJob) return 'A group update is already running. Wait for it to finish or cancel it safely.'
  if (state.isStarting) return 'The group update is starting.'
  if (state.selectedCount === 0) return 'Select at least one receiver.'
  if (state.isPackageCheckRunning) return 'Wait until the firmware package check is complete.'
  if (state.sourceMode === 'catalog' && !state.releaseSelected) return 'Select a firmware release.'
  if (state.sourceMode === 'catalog' && !state.buildSelected) return 'Select a firmware build.'
  if (!state.packagesReady) {
    return state.sourceMode === 'catalog'
      ? 'Firmware is missing or ambiguous for one or more selected receiver models.'
      : 'Select one valid local firmware package for every selected receiver model.'
  }
  return null
}

export function getSingleUpdateBlocker(state: {
  sourceMode: FirmwareSourceMode
  receiverSelected: boolean
  modelReady: boolean
  releaseSelected: boolean
  buildSelected: boolean
  buildModeSelected: boolean
  packageReady: boolean
  isPackageCheckRunning: boolean
  hasCapacity: boolean
  receiverAlreadyRunning: boolean
  isStarting: boolean
}) {
  if (state.isStarting) return 'The receiver update is starting.'
  if (!state.receiverSelected) return 'Select one receiver.'
  if (state.receiverAlreadyRunning) return 'An update is already running for this receiver.'
  if (!state.hasCapacity) return 'Three receiver updates are already active. Wait for one to finish.'
  if (state.isPackageCheckRunning) return 'Wait until the firmware package check is complete.'
  if (state.sourceMode === 'release' && !state.modelReady) return 'Select a valid receiver model.'
  if (state.sourceMode === 'release' && !state.releaseSelected) return 'Select a firmware release.'
  if (state.sourceMode === 'release' && !state.buildSelected) return 'Select a firmware build.'
  if (state.sourceMode === 'release' && !state.buildModeSelected) return 'Wait until the selected build is ready.'
  if (!state.packageReady) {
    return state.sourceMode === 'release'
      ? 'No matching firmware package was found for the selected receiver model.'
      : 'Select a valid local firmware package.'
  }
  return null
}

export function getMultipleUpdateBlocker(state: {
  sourceMode: FirmwareSourceMode
  selectedCount: number
  modelsReady: boolean
  oneSharedLocalModel: boolean
  releaseSelected: boolean
  buildSelected: boolean
  buildModeSelected: boolean
  packagesReady: boolean
  isPackageCheckRunning: boolean
  hasRunningBatch: boolean
  hasCapacity: boolean
  isStarting: boolean
}) {
  if (state.hasRunningBatch) return 'A multiple receiver update is already running.'
  if (state.isStarting) return 'The multiple receiver update is starting.'
  if (state.selectedCount < 2) return 'Select at least two receivers.'
  if (!state.modelsReady) return 'Select a valid model for every selected receiver.'
  if (!state.hasCapacity) return 'Three receiver updates are already active. Wait for one to finish.'
  if (state.isPackageCheckRunning) return 'Wait until the firmware package check is complete.'
  if (state.sourceMode === 'local' && !state.oneSharedLocalModel) {
    return 'Local multiple update requires one shared receiver model.'
  }
  if (state.sourceMode === 'release' && !state.releaseSelected) return 'Select a firmware release.'
  if (state.sourceMode === 'release' && !state.buildSelected) return 'Select a firmware build.'
  if (state.sourceMode === 'release' && !state.buildModeSelected) return 'Wait until the selected build is ready.'
  if (!state.packagesReady) {
    return state.sourceMode === 'release'
      ? 'Firmware is missing for one or more selected receiver models.'
      : 'Select a valid local firmware package.'
  }
  return null
}
