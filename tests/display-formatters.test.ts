import assert from 'node:assert/strict'
import test from 'node:test'

import {
  formatBytes,
  formatDuration,
  formatNumber,
  formatProgressPercent,
} from '../src/displayFormatters.ts'

test('formats receiver values and missing values consistently', () => {
  assert.equal(formatNumber(null), '--')
  assert.equal(formatNumber(12.34, ' C'), '12.3 C')
  assert.equal(formatNumber(12), '12')
})

test('formats durations using the largest useful units', () => {
  assert.equal(formatDuration(undefined), '--')
  assert.equal(formatDuration(90), '1m')
  assert.equal(formatDuration(3660), '1h 1m')
  assert.equal(formatDuration(90000), '1d 1h')
})

test('formats byte sizes and clamps progress percentages', () => {
  assert.equal(formatBytes(1024), '1.0 KB')
  assert.equal(formatBytes(Number.POSITIVE_INFINITY), '--')
  assert.equal(formatProgressPercent(-10), '0.0%')
  assert.equal(formatProgressPercent(105), '100%')
})
