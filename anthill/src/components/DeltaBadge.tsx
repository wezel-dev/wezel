import { useTheme } from "../lib/theme";
import { MONO, fmtValue } from "../lib/format";

export function DeltaBadge({
  current,
  baseline,
  unit,
  style,
}: {
  current: number;
  baseline: number;
  unit?: string;
  style?: React.CSSProperties;
}) {
  const { C } = useTheme();
  const diff = current - baseline;
  const pct = baseline !== 0 ? Math.round((diff / baseline) * 100) : 0;
  const isRegression = diff > 0;
  const color = diff === 0 ? C.textDim : isRegression ? C.red : C.green;
  const sign = diff > 0 ? "+" : "";

  if (diff === 0) return null;

  return (
    <span
      style={{
        fontSize: 10,
        fontFamily: MONO,
        fontWeight: 600,
        color,
        padding: "1px 5px",
        borderRadius: 3,
        background: color + "15",
        border: `1px solid ${color}33`,
        whiteSpace: "nowrap",
        ...style,
      }}
    >
      {sign}
      {fmtValue(diff, unit)} ({sign}
      {pct}%)
    </span>
  );
}