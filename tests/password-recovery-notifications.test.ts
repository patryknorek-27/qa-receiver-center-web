import assert from 'node:assert/strict'
import test from 'node:test'
import { collectNewRecoveryNotices, type PasswordRecoveryNotice } from '../src/passwordRecoveryNotifications.ts'

const notice = (ip = '192.0.2.10', jobId = 'test-job'): PasswordRecoveryNotice => ({ ip, jobId, message: 'Password not confirmed' })

test('repeated polls and changed error text do not notify again', () => {
  const seen = new Set<string>()
  assert.equal(collectNewRecoveryNotices([notice()], seen).length, 1)
  for (let i = 0; i < 5; i += 1) assert.deepEqual(collectNewRecoveryNotices([notice()], seen), [])
  assert.deepEqual(collectNewRecoveryNotices([{ ...notice(), message: 'More detail' }], seen), [])
})

test('a group of incidents is collected once, including duplicate entries', () => {
  const seen = new Set<string>()
  const notices = [notice(), notice(), notice('192.0.2.11')]
  assert.equal(collectNewRecoveryNotices(notices, seen).length, 2)
  assert.deepEqual(collectNewRecoveryNotices(notices, seen), [])
})

test('another receiver and another job on the same IP are new incidents', () => {
  const seen = new Set<string>()
  collectNewRecoveryNotices([notice()], seen)
  assert.equal(collectNewRecoveryNotices([notice(), notice('192.0.2.11')], seen).length, 1)
  assert.equal(collectNewRecoveryNotices([notice('192.0.2.10', 'next-job')], seen).length, 1)
})

test('a temporary empty snapshot does not re-arm a previously acknowledged incident', () => {
  const seen = new Set<string>()
  collectNewRecoveryNotices([notice()], seen)
  assert.deepEqual(collectNewRecoveryNotices([], seen), [])
  assert.deepEqual(collectNewRecoveryNotices([notice()], seen), [])
})
