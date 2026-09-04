import assert from 'node:assert/strict'
import test from 'node:test'

import { getFirmwareWorkerCount } from '../src/firmwareConcurrency.ts'

test('parallel mode starts every selected receiver', () => {
  assert.equal(getFirmwareWorkerCount(true, 16), 16)
  assert.equal(getFirmwareWorkerCount(true, 25), 25)
  assert.equal(getFirmwareWorkerCount(true, 80), 25)
})

test('sequential mode starts one receiver at a time', () => {
  assert.equal(getFirmwareWorkerCount(false, 16), 1)
  assert.equal(getFirmwareWorkerCount(false, 0), 0)
})
