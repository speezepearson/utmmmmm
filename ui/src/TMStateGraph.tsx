import {
  type Dir,
  type State,
  type Symbol,
  type TuringMachineSpec,
} from "./types";

type Props = {
  spec: TuringMachineSpec;
  state: State;
};

type TargetNode = {
  id: string;
  label: string;
  symbols: Symbol[];
};

export function TMStateGraph({ spec, state }: Props) {
  const rulesForState = spec.rules.get(state);
  if (!rulesForState)
    return (
      <div className="tm-state-graph-empty">No transitions from this state</div>
    );

  // Categorize transitions: identity (same state, same symbol) vs normal
  const identity: Record<Dir, Symbol[]> = { L: [], R: [] };
  const normal: {
    symbol: Symbol;
    newState: State;
    newSymbol: Symbol;
    dir: Dir;
  }[] = [];

  for (const [symbol, [newState, newSymbol, dir]] of rulesForState) {
    if (newState === state && newSymbol === symbol) {
      identity[dir].push(symbol);
    } else {
      normal.push({ symbol, newState, newSymbol, dir });
    }
  }

  // Build target nodes
  const targets: TargetNode[] = [];

  for (const dir of ["L", "R"] as Dir[]) {
    if (identity[dir].length > 0) {
      targets.push({ id: `identity-${dir}`, label: dir, symbols: identity[dir] });
    }
  }

  for (const t of normal) {
    targets.push({
      id: `normal-${String(t.symbol)}`,
      label: `write ${String(t.newSymbol)}\nmove ${t.dir}\n→ ${String(t.newState)}`,
      symbols: [t.symbol],
    });
  }

  if (targets.length === 0) return null;

  // Layout constants
  const srcNodeW = 100;
  const srcNodeH = 40;
  const tgtNodeW = 130;
  const tgtLineH = 18;
  const tgtPadY = 14;
  const rowGap = 12;
  const gapX = 120;
  const padX = 20;
  const padY = 16;

  // Measure target node heights
  const tgtHeights = targets.map(
    (t) => t.label.split("\n").length * tgtLineH + tgtPadY,
  );
  const totalTgtH =
    tgtHeights.reduce((a, b) => a + b, 0) + rowGap * (targets.length - 1);

  const svgW = padX + srcNodeW + gapX + tgtNodeW + padX;
  const svgH = Math.max(srcNodeH + 2 * padY, totalTgtH + 2 * padY);

  const srcCX = padX + srcNodeW / 2;
  const srcCY = svgH / 2;
  const tgtLeft = padX + srcNodeW + gapX;
  const tgtCX = tgtLeft + tgtNodeW / 2;

  // Compute target node Y centers
  let curY = (svgH - totalTgtH) / 2;
  const tgtCYs: number[] = [];
  for (let i = 0; i < targets.length; i++) {
    tgtCYs.push(curY + tgtHeights[i] / 2);
    curY += tgtHeights[i] + rowGap;
  }

  const fmtSymbols = (syms: Symbol[]) => syms.map(String).join(", ");

  return (
    <svg
      className="tm-state-graph"
      width={svgW}
      height={svgH}
      style={{ display: "block", margin: "8px 0" }}
    >
      <defs>
        <marker
          id="sg-arrow"
          markerWidth="8"
          markerHeight="6"
          refX="8"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="var(--text, #888)" />
        </marker>
      </defs>

      {/* Source node */}
      <rect
        x={srcCX - srcNodeW / 2}
        y={srcCY - srcNodeH / 2}
        width={srcNodeW}
        height={srcNodeH}
        rx="6"
        fill="var(--code-bg, #f5f5f5)"
        stroke="#6366f1"
        strokeWidth="2"
      />
      <text
        x={srcCX}
        y={srcCY}
        textAnchor="middle"
        dominantBaseline="central"
        fill="var(--text-h, #222)"
        fontFamily="var(--mono, monospace)"
        fontSize="14"
        fontWeight="700"
      >
        {String(state)}
      </text>

      {/* Edges and target nodes */}
      {targets.map((target, i) => {
        const ty = tgtCYs[i];
        const h = tgtHeights[i];
        const lines = target.label.split("\n");

        // Bezier from source right edge to target left edge
        const x1 = srcCX + srcNodeW / 2;
        const y1 = srcCY;
        const x2 = tgtLeft;
        const y2 = ty;
        const cpx = (x1 + x2) / 2;

        // Label position along the curve
        const lx = cpx;
        const ly = (y1 + y2) / 2 - 8;

        return (
          <g key={target.id}>
            {/* Arrow */}
            <path
              d={`M${x1},${y1} C${cpx},${y1} ${cpx},${y2} ${x2},${y2}`}
              fill="none"
              stroke="var(--text, #888)"
              strokeWidth="1.5"
              markerEnd="url(#sg-arrow)"
            />
            {/* Edge label (symbols) */}
            <text
              x={lx}
              y={ly}
              textAnchor="middle"
              fill="var(--text, #888)"
              fontFamily="var(--mono, monospace)"
              fontSize="11"
            >
              {fmtSymbols(target.symbols)}
            </text>

            {/* Target node */}
            <rect
              x={tgtLeft}
              y={ty - h / 2}
              width={tgtNodeW}
              height={h}
              rx="6"
              fill="var(--code-bg, #f5f5f5)"
              stroke="var(--border, #ccc)"
              strokeWidth="1.5"
            />
            {lines.map((line, li) => (
              <text
                key={li}
                x={tgtCX}
                y={ty - ((lines.length - 1) * tgtLineH) / 2 + li * tgtLineH}
                textAnchor="middle"
                dominantBaseline="central"
                fill="var(--text-h, #222)"
                fontFamily="var(--mono, monospace)"
                fontSize="12"
              >
                {line}
              </text>
            ))}
          </g>
        );
      })}
    </svg>
  );
}
