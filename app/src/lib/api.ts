export async function invoke<T>(cmd: string, args?: any): Promise<T> {
  // Placeholder: replace with Tauri invoke when wired
  const res = await fetch(`/api/${cmd}`, { method: 'POST', body: JSON.stringify(args||{} )})
  return res.json()
}