import type { Theme } from "./types";

/** Apply the configured colour scheme to the document (CSS reads data-theme). */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  if (theme === "auto") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);
}
