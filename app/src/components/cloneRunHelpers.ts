export function isCloneRunDisabled(
  selectedRunId: string | null,
  runActionPending: boolean,
  checkpointCount: number,
): boolean {
  if (!selectedRunId) {
    return true;
  }
  if (runActionPending) {
    return true;
  }
  return checkpointCount === 0;
}
