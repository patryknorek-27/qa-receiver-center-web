import assert from 'node:assert/strict'
import test from 'node:test'
import { releaseMatchesMode } from '../src/firmwareReleaseSelection.ts'

test('a shared release appears in both Alpha and Beta lists', () => {
  const name = 'V6.70 [ Alpha releases on main. Beta releases on 67X branch ]'
  assert.equal(releaseMatchesMode(name, 'Alpha'), true)
  assert.equal(releaseMatchesMode(name, 'Beta'), true)
})

test('a release for one channel stays in that channel', () => {
  assert.equal(releaseMatchesMode('V6.80 [ Alpha releases on main ]', 'Alpha'), true)
  assert.equal(releaseMatchesMode('V6.80 [ Alpha releases on main ]', 'Beta'), false)
  assert.equal(releaseMatchesMode('V6.65 [ Special Beta releases ]', 'Alpha'), false)
  assert.equal(releaseMatchesMode('V6.65 [ Special Beta releases ]', 'Beta'), true)
})

test('a release without a channel remains available for build-level matching', () => {
  assert.equal(releaseMatchesMode('V7.00 [ Future releases from main branch ]', 'Alpha'), true)
  assert.equal(releaseMatchesMode('V7.00 [ Future releases from main branch ]', 'Beta'), true)
})
