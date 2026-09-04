import assert from 'node:assert/strict'
import test from 'node:test'

import { restoreReceiverSelection } from '../src/firmwareSelection.ts'

test('returning to a group preserves deselected receivers', () => {
  const receivers = ['10.2.155.11', '10.2.155.12', '10.2.155.13']
  const remembered = new Set(['10.2.155.11', '10.2.155.13'])

  assert.deepEqual(Array.from(restoreReceiverSelection(receivers, remembered)), [
    '10.2.155.11',
    '10.2.155.13',
  ])
})

test('an intentionally empty selection remains empty', () => {
  assert.equal(restoreReceiverSelection(['10.2.155.11'], new Set()).size, 0)
})

test('the first group visit selects available receivers', () => {
  assert.deepEqual(Array.from(restoreReceiverSelection(['10.2.155.11', '10.2.155.12'], undefined)), [
    '10.2.155.11',
    '10.2.155.12',
  ])
})
