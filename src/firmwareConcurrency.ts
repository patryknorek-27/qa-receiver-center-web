export const MAX_PARALLEL_FIRMWARE_RECEIVERS = 25

export function getFirmwareWorkerCount(parallel: boolean, receiverCount: number) {
  if (receiverCount <= 0) return 0
  return parallel ? Math.min(receiverCount, MAX_PARALLEL_FIRMWARE_RECEIVERS) : 1
}
