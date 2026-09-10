import assert from 'node:assert/strict'
import test from 'node:test'

import {
  getGroupUpdateBlocker,
  getMultipleUpdateBlocker,
  getSingleUpdateBlocker,
} from '../src/firmwareStartBlockers.ts'

test('group update reports a missing firmware package', () => {
  assert.equal(
    getGroupUpdateBlocker({
      isGroupUpdate: true,
      selectedCount: 3,
      hasActiveJob: false,
      isStarting: false,
      sourceMode: 'catalog',
      releaseSelected: true,
      buildSelected: true,
      packagesReady: false,
      isPackageCheckRunning: false,
    }),
    'Firmware is missing or ambiguous for one or more selected receiver models.',
  )
})

test('single update reports why start is blocked and becomes ready', () => {
  const state = {
    sourceMode: 'release' as const,
    receiverSelected: true,
    modelReady: true,
    releaseSelected: true,
    buildSelected: true,
    buildModeSelected: true,
    packageReady: false,
    isPackageCheckRunning: false,
    hasCapacity: true,
    receiverAlreadyRunning: false,
    isStarting: false,
  }
  assert.equal(
    getSingleUpdateBlocker(state),
    'No matching firmware package was found for the selected receiver model.',
  )
  assert.equal(getSingleUpdateBlocker({ ...state, packageReady: true }), null)
})

test('multiple update requires at least two receivers', () => {
  assert.equal(
    getMultipleUpdateBlocker({
      sourceMode: 'local',
      selectedCount: 1,
      modelsReady: true,
      oneSharedLocalModel: true,
      releaseSelected: false,
      buildSelected: false,
      buildModeSelected: false,
      packagesReady: true,
      isPackageCheckRunning: false,
      hasRunningBatch: false,
      hasCapacity: true,
      isStarting: false,
    }),
    'Select at least two receivers.',
  )
})
