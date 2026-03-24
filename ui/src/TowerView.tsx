import { useEffect, useRef, useState } from "react";

const GREEN_SYMS = new Set(["*", "X", "Y", "^", ">"]);

interface TowerViewState {
  steps: number;
  guestSteps: number;
  stepsPerSec: number;
  tower: Array<{
    state: string;
    headPos: number;
    maxHeadPos: number;
    tape: string;
    tapeLen: number;
  }>;
}

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

export function TowerView() {
  const [data, setData] = useState<TowerViewState | null>(null);
  const tapesRef = useRef<string[]>([]);
  const unblemishedRef = useRef<string>("");

  useEffect(() => {
    const es = new EventSource("/api/tower");
    es.onmessage = (event) => {
      const msg = JSON.parse(event.data);

      if (msg.type === "total") {
        // Store unblemished reference tape
        unblemishedRef.current = msg.unblemished;
        // Initialize tapes from total event
        tapesRef.current = msg.tower.map(
          (l: { tape: string }) => l.tape,
        );
        setData({
          steps: msg.steps,
          guestSteps: msg.guest_steps,
          stepsPerSec: msg.steps_per_sec,
          tower: msg.tower.map(
            (
              l: {
                state: string;
                head_pos: number;
                max_head_pos: number;
                tape: string;
                tape_len: number;
              },
            ) => ({
              state: l.state,
              headPos: l.head_pos,
              maxHeadPos: l.max_head_pos,
              tape: l.tape,
              tapeLen: l.tape_len,
            }),
          ),
        });
      } else if (msg.type === "delta") {
        // Apply new_overwrites to stored tapes
        for (let i = 0; i < msg.tower.length; i++) {
          const level = msg.tower[i];
          let tape = tapesRef.current[i] || "";
          const chars = tape.split("");
          // Extend tape to max_head_pos + 10 using unblemished content
          const end = Math.max(level.max_head_pos, level.head_pos) + 10;
          const ub = unblemishedRef.current;
          while (chars.length < end) {
            const pos = chars.length;
            chars.push(pos < ub.length ? ub[pos] : "_");
          }
          for (const [pos, ch] of level.new_overwrites as [
            number,
            string,
          ][]) {
            while (chars.length <= pos) chars.push("_");
            chars[pos] = ch;
          }
          tapesRef.current[i] = chars.join("");
        }
        // Trim if tower shrank
        tapesRef.current.length = msg.tower.length;

        setData({
          steps: msg.total_steps,
          guestSteps: msg.guest_steps,
          stepsPerSec: msg.steps_per_sec,
          tower: msg.tower.map(
            (
              l: {
                state: string;
                head_pos: number;
                max_head_pos: number;
                new_overwrites: [number, string][];
                tape_len: number;
              },
              i: number,
            ) => ({
              state: l.state,
              headPos: l.head_pos,
              maxHeadPos: l.max_head_pos,
              tape: tapesRef.current[i],
              tapeLen: l.tape_len,
            }),
          ),
        });
      }
    };
    return () => es.close();
  }, []);

  if (!data) {
    return <div style={{ padding: "16px" }}>Loading...</div>;
  }

  return (
    <div style={{ textAlign: "left", padding: "16px" }}>
      <h2 style={{ marginBottom: "8px" }}>
        Tower &mdash; {data.steps.toLocaleString()} steps
        {data.stepsPerSec > 0 && (
          <span
            style={{
              fontWeight: "normal",
              fontSize: "14px",
              marginLeft: "12px",
            }}
          >
            ({data.stepsPerSec.toFixed(1)}M steps/s,{" "}
            {data.guestSteps.toLocaleString()} guest steps)
          </span>
        )}
      </h2>
      <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
        {data.tower.map((level, i) => {
          const end = Math.max(level.maxHeadPos, level.headPos) + 10;
          const tape = level.tape.slice(0, end);
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
                L{i} &middot; {level.state} &middot;{" "}
                {level.tapeLen.toLocaleString()} symbols
              </div>
              <div
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: "12px",
                  lineHeight: "1.3",
                  overflowWrap: "break-word",
                }}
                dangerouslySetInnerHTML={{
                  __html: colorizeTape(tape, level.headPos),
                }}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
