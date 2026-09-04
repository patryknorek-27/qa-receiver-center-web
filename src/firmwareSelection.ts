export function restoreReceiverSelection(
  receiverIps: readonly string[],
  rememberedSelection: ReadonlySet<string> | undefined,
) {
  if (!rememberedSelection) return new Set(receiverIps)
  const available = new Set(receiverIps)
  return new Set(Array.from(rememberedSelection).filter((ip) => available.has(ip)))
}
