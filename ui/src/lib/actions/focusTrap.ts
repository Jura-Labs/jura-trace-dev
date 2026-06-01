// SPDX-License-Identifier: AGPL-3.0-or-later
/**
 * focusTrap — Svelte action for modal focus management (WCAG 2.2 AA).
 *
 * On mount:
 *  - Remembers the previously focused element.
 *  - Moves focus to the first focusable child, or the node itself if none exist.
 *
 * While active:
 *  - Traps Tab / Shift+Tab within the node boundary.
 *  - Fires the optional onEscape callback when Escape is pressed.
 *
 * On destroy:
 *  - Restores focus to the element that held it before the action mounted.
 *
 * Usage:
 *   <div use:focusTrap={{ onEscape: () => open = false }}>…</div>
 */

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(', ');

function getFocusable(node: HTMLElement): HTMLElement[] {
  return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => el.offsetParent !== null,
  );
}

interface FocusTrapOptions {
  /** Called when Escape is pressed inside the trap. Use to close the dialog. */
  onEscape?: () => void;
}

export function focusTrap(node: HTMLElement, options: FocusTrapOptions = {}) {
  const previouslyFocused = document.activeElement as HTMLElement | null;

  // Ensure the node itself is focusable so we have somewhere to land when
  // the inner focusable list is empty (e.g. a loading overlay).
  if (!node.hasAttribute('tabindex')) {
    node.setAttribute('tabindex', '-1');
  }

  // Move focus in on the next microtask so the DOM is fully painted.
  Promise.resolve().then(() => {
    const first = getFocusable(node)[0];
    (first ?? node).focus();
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      options.onEscape?.();
      return;
    }

    if (event.key !== 'Tab') return;

    const focusable = getFocusable(node);
    if (focusable.length === 0) {
      event.preventDefault();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (event.shiftKey) {
      if (document.activeElement === first || document.activeElement === node) {
        event.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
  }

  node.addEventListener('keydown', handleKeydown);

  return {
    update(newOptions: FocusTrapOptions) {
      options = newOptions;
    },
    destroy() {
      node.removeEventListener('keydown', handleKeydown);
      previouslyFocused?.focus();
    },
  };
}
