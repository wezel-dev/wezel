import { useCallback, useEffect, useRef } from "react";

type DragCallback = (
  dx: number,
  dy: number,
  clientX: number,
  clientY: number,
) => void;

interface UseDragOptions {
  onDrag: DragCallback;
  onDragEnd?: () => void;
  cursor?: string;
}

/**
 * Shared hook that encapsulates the common mouse-drag pattern:
 *  - attaches mousemove/mouseup on document during drag
 *  - resets cursor & userSelect on body
 *  - reports deltas via onDrag callback
 *
 * Returns an `onMouseDown` handler to attach to the drag-handle element.
 */
export function useDrag({
  onDrag,
  onDragEnd,
  cursor = "col-resize",
}: UseDragOptions) {
  // Store callbacks in refs so the mousemove/mouseup closures always see the
  // latest values without needing to re-create the onMouseDown handler.
  const onDragRef = useRef(onDrag);
  const onDragEndRef = useRef(onDragEnd);
  const cursorRef = useRef(cursor);

  useEffect(() => {
    onDragRef.current = onDrag;
    onDragEndRef.current = onDragEnd;
    cursorRef.current = cursor;
  });

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    let lastX = e.clientX;
    let lastY = e.clientY;

    const onMouseMove = (ev: MouseEvent) => {
      const dx = ev.clientX - lastX;
      const dy = ev.clientY - lastY;
      lastX = ev.clientX;
      lastY = ev.clientY;
      onDragRef.current(dx, dy, ev.clientX, ev.clientY);
    };

    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      onDragEndRef.current?.();
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    document.body.style.cursor = cursorRef.current;
    document.body.style.userSelect = "none";
  }, []);

  return onMouseDown;
}
