export async function invoke(command, args = {}) {
  const api = window.__TAURI_INTERNALS__;
  if (!api?.invoke) throw new Error("Tauri API unavailable: run inside the desktop app");
  return api.invoke(command, args);
}

export async function listen(event, handler) {
  const api = window.__TAURI_INTERNALS__;
  if (!api?.listen) return () => {};
  return api.listen(event, handler);
}
