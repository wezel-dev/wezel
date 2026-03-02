import { useState, useCallback } from "react";
import { useTheme } from "../lib/theme";
import { useDrag } from "../lib/useDrag";

export function PanelHandle({ onDrag }: { onDrag: (delta: number) => void }) {
  const { C } = useTheme();
  const [hover, setHover] = useState(false);

  const onMouseDown = useDrag({
    onDrag: useCallback((dx: number) => onDrag(dx), [onDrag]),
    cursor: "col-resize",
  });

  return (
    <div
      onMouseDown={onMouseDown}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        width: 6,
        flexShrink: 0,
        cursor: "col-resize",
        display: "flex",
        justifyContent: "center",
        background: hover ? C.accent + "22" : "transparent",
        transition: "background 0.1s",
      }}
    >
      <div
        style={{
          width: 1,
          height: "100%",
          background: hover ? C.accent : C.border,
          transition: "background 0.1s",
        }}
      />
    </div>
  );
}
