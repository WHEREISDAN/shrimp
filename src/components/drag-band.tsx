/**
 * The draggable top band of a screen outside the app shell, such as onboarding
 * and the error screens: with no title bar, it is where the window is moved.
 */
export function DragBand() {
  return (
    <div
      aria-hidden="true"
      data-tauri-drag-region
      className="fixed inset-x-0 top-0 z-40 h-(--toolbar-h) windows:h-8"
    />
  );
}
