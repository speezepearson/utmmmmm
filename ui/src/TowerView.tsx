import { z } from "zod";
import { useEffect, useRef, useState } from "react";
import { updateTower, type TowerLevel, type UtmMeta } from "./tower";

const GREEN_SYMS = new Set(["*", "X", "Y", "^", ">"]);

function colorizeTape(tape: string, headPos: number): string {
  let out = "";
  for (let i = 0; i < tape.length; i++) {
    const ch = tape[i];
    const escaped =
      ch === "&" ? "&amp;" : ch === "<" ? "&lt;" : ch === ">" ? "&gt;" : ch;
    if (i === headPos) {
      out += `<span style="background:#f87171">${escaped}</span>`;
    } else if (GREEN_SYMS.has(ch)) {
      out += `<span style="background:#4ade80">${escaped}</span>`;
    } else {
      out += escaped;
    }
  }
  return out;
}

// ── L0 state from server ──

interface L0State {
  steps: number;
  state: string;
  headPos: number;
  tape: string;
}

const TotalEvent = z.object({
  type: z.literal("total"),
  steps: z.number(),
  unblemished: z.string(),
  utm_states: z.array(z.string()),
  utm_symbol_chars: z.string(),
  state: z.string(),
  head_pos: z.number(),
  overwrites: z.record(z.number(), z.string()),
});
type TotalEvent = z.infer<typeof TotalEvent>;
const DeltaEvent = z.object({
  type: z.literal("delta"),
  total_steps: z.number(),
  state: z.string(),
  head_pos: z.number(),
  new_overwrites: z.record(z.number(), z.string()),
});
type DeltaEvent = z.infer<typeof DeltaEvent>;
const SseEvent = z.union([TotalEvent, DeltaEvent]);

function useSseL0(): { meta: UtmMeta | null; l0: L0State | null } {
  const unblemishedRef = useRef<string>("");
  const [meta, setMeta] = useState<UtmMeta | null>(null);

  const l0Ref = useRef<L0State | null>(null);
  const [exposedL0, setExposedL0] = useState<L0State | null>(null);

  useEffect(() => {
    const es = new EventSource("/api/tower");
    es.onmessage = (event) => {
      const msg = SseEvent.parse(JSON.parse(event.data));

      switch (msg.type) {
        case "total": {
          unblemishedRef.current = msg.unblemished;
          setMeta({
            utmStates: msg.utm_states,
            utmSymbolChars: msg.utm_symbol_chars,
          });
          l0Ref.current = {
            steps: msg.steps,
            state: msg.state,
            headPos: msg.head_pos,
            tape: Array.from(
              {
                length: Math.max(
                  msg.head_pos,
                  ...Object.keys(msg.overwrites).map(Number),
                ),
              },
              (_, i) => msg.overwrites[i] ?? unblemishedRef.current[i] ?? "_",
            ).join(""),
          };
          setExposedL0(l0Ref.current);
          break;
        }
        case "delta": {
          l0Ref.current = {
            steps: msg.total_steps,
            state: msg.state,
            headPos: msg.head_pos,
            tape: Array.from(
              {
                length: Math.max(
                  l0Ref.current?.tape.length ?? 0,
                  msg.head_pos,
                  ...Object.keys(msg.new_overwrites).map(Number),
                ),
              },
              (_, i) =>
                msg.new_overwrites[i] ??
                l0Ref.current?.tape[i] ??
                unblemishedRef.current[i] ??
                "_",
            ).join(""),
          };
          setExposedL0(l0Ref.current);
          break;
        }
      }
    };
    return () => es.close();
  }, []);

  return {
    meta,
    l0: exposedL0,
  };
}

// ── Main component ──

export function TowerView() {
  const { meta, l0 } = useSseL0();
  const towerRef = useRef<TowerLevel[]>([]);
  const [tower, setTower] = useState<TowerLevel[] | null>(null);

  useEffect(() => console.log({ l0 }), [l0]);
  useEffect(() => console.log({ tower }), [tower]);

  useEffect(() => {
    if (l0 && meta) updateTower(l0, towerRef.current, meta);
    setTower([...towerRef.current]);
  }, [l0, meta]);

  if (!l0 || !tower) {
    return <div style={{ padding: "16px" }}>Loading...</div>;
  }

  return (
    <div style={{ textAlign: "left", padding: "16px" }}>
      <h2 style={{ marginBottom: "8px" }}>
        Tower &mdash; {l0.steps.toLocaleString()} steps
      </h2>
      <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
        {tower.map((level, i) => {
          return (
            <div
              key={i}
              style={{
                background: "var(--code-bg)",
                padding: "8px 12px",
                borderRadius: "6px",
                transition: "height 0.3s ease, min-height 0.3s ease",
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  fontSize: "11px",
                  color: "#888",
                  marginBottom: "4px",
                }}
              >
                L{i} &middot; {level.state}
              </div>
              <div
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: "12px",
                  lineHeight: "1.3",
                  overflowWrap: "break-word",
                }}
                dangerouslySetInnerHTML={{
                  __html: colorizeTape(level.tape, level.headPos),
                }}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
