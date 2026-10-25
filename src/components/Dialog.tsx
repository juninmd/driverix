import { type ReactNode, useEffect, useRef } from "react";

interface Props {
  title: string;
  onClose: () => void;
  children: ReactNode;
  actions: ReactNode;
}

const FOCUSABLE = 'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex="0"]';

/**
 * Marks everything outside the path from `node` up to <html> as inert. The dialog can render inside
 * a tab panel, so its direct siblings are not the whole page; the header and tabs sit further up.
 */
function inertOutside(node: Element): () => void {
  const marked: Element[] = [];
  for (let el: Element = node; el.parentElement; el = el.parentElement) {
    for (const sib of el.parentElement.children) {
      if (sib !== el && !sib.hasAttribute("inert")) {
        sib.setAttribute("inert", "");
        marked.push(sib);
      }
    }
  }
  return () => {
    for (const el of marked) el.removeAttribute("inert");
  };
}

export function Dialog({ title, onClose, children, actions }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  // Held in a ref so the focus effect below runs once; a new onClose identity would otherwise re-focus.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const dialog = ref.current;
    dialog?.focus();
    const releaseInert = dialog ? inertOutside(dialog) : () => {};

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeRef.current();
      if (e.key !== "Tab" || !dialog) return;
      const items = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)];
      if (items.length === 0) return;
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey && (active === first || active === dialog)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      releaseInert();
      previous?.focus();
    };
  }, []);

  return (
    <div className="backdrop">
      <div ref={ref} className="dialog pop-in" role="dialog" aria-modal="true" aria-label={title} tabIndex={-1}>
        <h2>{title}</h2>
        <div className="dialog-body">{children}</div>
        <div className="dialog-actions">{actions}</div>
      </div>
    </div>
  );
}
