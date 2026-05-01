/// Try to write text to the system clipboard. Returns true on success,
/// false on failure (permission denied, insecure context, lost focus).
/// Always logs failures to `console.error` so a call site that doesn't
/// surface a UI fallback is still diagnosable from the webview devtools.
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch (e) {
    console.error("clipboard write failed:", e);
    return false;
  }
}
