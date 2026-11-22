// Billy's dialogs block the window's content but not the title bar, so the window can still
// be moved, maximized, minimized and closed while one is open. A native modal dialog
// (showModal) would block the title bar too, so dialogs are opened with show() and the page
// makes the rest of its content inert and dims it while `modal.open` is above zero.

export const modal = $state({ open: 0 });

/** Open dialogs, the topmost last. */
const stack: HTMLDialogElement[] = [];

function onKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape" || event.defaultPrevented) return;
  const top = stack.at(-1);
  if (!top) return;
  event.preventDefault();
  top.close();
}

function remove(dialog: HTMLDialogElement) {
  const i = stack.indexOf(dialog);
  if (i < 0) return;
  stack.splice(i, 1);
  modal.open = stack.length;
  // The dialog underneath, if any, can be used again.
  stack.at(-1)?.removeAttribute("inert");
  if (stack.length === 0) window.removeEventListener("keydown", onKeydown);
}

/**
 * Opens a dialog over the window's content. Escape closes the topmost one, wherever the
 * focus is. Use it from an effect: `$effect(() => openDialog(dialog))`; the returned
 * function forgets the dialog if its component goes away without closing it.
 */
export function openDialog(dialog: HTMLDialogElement): () => void {
  stack.at(-1)?.setAttribute("inert", "");
  stack.push(dialog);
  modal.open = stack.length;
  if (stack.length === 1) window.addEventListener("keydown", onKeydown);
  dialog.addEventListener("close", () => remove(dialog), { once: true });
  dialog.show();
  return () => remove(dialog);
}
