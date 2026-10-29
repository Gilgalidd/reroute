// Writing to the clipboard from the page, without a Tauri plugin: the web
// view's own clipboard does it, and nothing is ever read from it.

/**
 * Put `text` on the clipboard. The asynchronous Clipboard API first; a
 * web view that lacks it or refuses it gets the older `copy` command on a
 * hidden field, as WebKitGTK may. Throws when neither worked.
 */
export async function copyText(text: string): Promise<void> {
  try {
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(text);
      return;
    }
  } catch {
    // Refused: try the older way below.
  }
  if (!copyThroughField(text)) throw new Error("the clipboard refused the link");
}

function copyThroughField(text: string): boolean {
  if (typeof document.execCommand !== "function") return false;
  const field = document.createElement("textarea");
  field.value = text;
  field.readOnly = true;
  // Through the CSSOM, which the CSP allows, unlike a style attribute.
  field.style.position = "fixed";
  field.style.opacity = "0";
  document.body.append(field);
  try {
    field.select();
    return document.execCommand("copy");
  } finally {
    field.remove();
  }
}
