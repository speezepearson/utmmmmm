import { useEffect, useRef, useState, useMemo } from "react";

// ── L0 state from server ──

interface L0State {
  steps: number;
  guestSteps: number;
  stepsPerSec: number;
  state: string;
  headPos: number;
  maxHeadPos: number;
  tape: string;
  tapeLen: number;
}

interface UtmMeta {
  utmStates: string[];
  utmSymbolChars: string;
}

// ── Client-side UTM tape decoder ──

function numBits(count: number): number {
  return Math.max(1, Math.ceil(Math.log2(Math.max(count, 2))));
}

function fromBinary(s: string, start: number, width: number): number | null {
  let val = 0;
  for (let i = start; i < start + width; i++) {
    const ch = s[i];
    if (ch === "0" || ch === "X") val = val * 2;
    else if (ch === "1" || ch === "Y") val = val * 2 + 1;
    else return null; // not valid binary
  }
  return val;
}

interface DecodedLevel {
  state: string;
  headPos: number;
  tape: string;
}

function decodeUtmTape(
  tape: string,
  utmStates: string[],
  utmSymbolChars: string,
): DecodedLevel | null {
  // Find 5 # delimiters
  const hashes: number[] = [];
  for (let i = 0; i < tape.length; i++) {
    if (tape[i] === "#") {
      hashes.push(i);
      if (hashes.length >= 5) break;
    }
  }
  if (hashes.length < 5) return null;

  const rulesStr = tape.slice(hashes[0] + 1, hashes[1]);
  const stateStr = tape.slice(hashes[2] + 1, hashes[3]);
  const tapeStr = tape.slice(hashes[4] + 1);

  // Determine bit widths from UTM metadata
  const nStateBits = numBits(utmStates.length);
  const nSymBits = numBits(utmSymbolChars.length);

  // Validate: also check against rule structure if rules exist
  if (rulesStr.length > 0) {
    const firstRule = rulesStr.split(";")[0];
    if (firstRule.length > 1) {
      // Rule format: .STATEBITS|SYMBITS|STATEBITS|SYMBITS|DIR
      const content = firstRule.slice(1); // skip . or *
      const pipes = content.split("|");
      if (pipes.length >= 2) {
        // Cross-check: rule field widths should match our computed bit widths
        if (pipes[0].length !== nStateBits || pipes[1].length !== nSymBits) {
          return null; // bit width mismatch — this isn't a self-simulation tape
        }
      }
    }
  }

  // Decode state
  if (stateStr.length !== nStateBits) return null;
  const stateIdx = fromBinary(stateStr, 0, nStateBits);
  if (stateIdx === null || stateIdx >= utmStates.length) return null;
  const stateName = utmStates[stateIdx];

  // Decode tape section (matching Rust decode logic)
  const cells: number[] = [];
  let headPos = 0;
  let cellIdx = 0;
  let i = 0;
  while (i < tapeStr.length) {
    const ch = tapeStr[i];
    if (ch === "_" || ch === "$") break;
    if (ch === ",") {
      cellIdx++;
      i++;
      continue;
    }
    if (ch === "^" || ch === ">") {
      if (ch === "^") headPos = cellIdx;
      i++;
      continue;
    }
    // Read nSymBits binary digits
    if (i + nSymBits > tapeStr.length) break;
    const val = fromBinary(tapeStr, i, nSymBits);
    if (val === null || val >= utmSymbolChars.length) break;
    cells.push(val);
    i += nSymBits;
  }

  if (cells.length === 0) return null;

  // Map cell indices to symbol characters
  const decodedTape = cells.map((idx) => utmSymbolChars[idx]).join("");

  return { state: stateName, headPos, tape: decodedTape };
}

// ── Build tower by recursively decoding ──

interface TowerLevel {
  state: string;
  headPos: number;
  tape: string;
  tapeLen: number;
}

function buildTower(
  l0: L0State,
  meta: UtmMeta | null,
): TowerLevel[] {
  const levels: TowerLevel[] = [
    {
      state: l0.state,
      headPos: l0.headPos,
      tape: l0.tape,
      tapeLen: l0.tapeLen,
    },
  ];

  if (!meta) return levels;

  let currentTape = l0.tape;
  for (let depth = 0; depth < 10; depth++) {
    const decoded = decodeUtmTape(
      currentTape,
      meta.utmStates,
      meta.utmSymbolChars,
    );
    if (!decoded) break;
    levels.push({
      state: decoded.state,
      headPos: decoded.headPos,
      tape: decoded.tape,
      tapeLen: decoded.tape.length,
    });
    currentTape = decoded.tape;
  }

  return levels;
}

// ── Semantic tape display ──

interface TapeSections {
  prefix: string;
  rules: string[];
  accepting: string;
  state: string;
  blank: string;
  tapeCells: string[];
}

function parseTape(tape: string): TapeSections | null {
  // Tape format: $#.rule1;.rule2;*activeRule#accepting#state#blank#^cell,cell,...
  const hashPositions: number[] = [];
  for (let i = 0; i < tape.length; i++) {
    if (tape[i] === "#") hashPositions.push(i);
  }
  if (hashPositions.length < 5) return null;

  const prefix = tape.slice(0, hashPositions[0]);
  const rulesStr = tape.slice(hashPositions[0] + 1, hashPositions[1]);
  const accepting = tape.slice(hashPositions[1] + 1, hashPositions[2]);
  const state = tape.slice(hashPositions[2] + 1, hashPositions[3]);
  const blank = tape.slice(hashPositions[3] + 1, hashPositions[4]);
  const tapeStr = tape.slice(hashPositions[4] + 1);

  const rules = rulesStr.length > 0 ? rulesStr.split(";") : [];
  const tapeCells = tapeStr.length > 0 ? tapeStr.split(",") : [];

  return { prefix, rules, accepting, state, blank, tapeCells };
}

function HeadChar({ ch }: { ch: string }) {
  return <span className="st-head">{ch}</span>;
}

function CharSpan({
  text,
  headPos,
  startIdx,
}: {
  text: string;
  headPos: number;
  startIdx: number;
}) {
  const parts: React.ReactNode[] = [];
  let run = "";
  let runStart = startIdx;
  for (let i = 0; i < text.length; i++) {
    const globalIdx = startIdx + i;
    if (globalIdx === headPos) {
      if (run) parts.push(<span key={runStart}>{run}</span>);
      parts.push(<HeadChar key={globalIdx} ch={text[i]} />);
      run = "";
      runStart = globalIdx + 1;
    } else {
      run += text[i];
    }
  }
  if (run) parts.push(<span key={runStart}>{run}</span>);
  return <>{parts}</>;
}

function SemanticTape({
  tape,
  headPos,
}: {
  tape: string;
  headPos: number;
}) {
  const parsed = useMemo(() => parseTape(tape), [tape]);

  if (!parsed) {
    return <CharSpan text={tape} headPos={headPos} startIdx={0} />;
  }

  let pos = parsed.prefix.length;
  const rulesStart = pos + 1;

  const rulePositions: number[] = [];
  let rp = rulesStart;
  for (let i = 0; i < parsed.rules.length; i++) {
    rulePositions.push(rp);
    rp += parsed.rules[i].length;
    if (i < parsed.rules.length - 1) rp += 1;
  }
  const afterRules = rp + 1;

  const acceptingStart = afterRules;
  const afterAccepting = acceptingStart + parsed.accepting.length + 1;

  const stateStart = afterAccepting;
  const afterState = stateStart + parsed.state.length + 1;

  const blankStart = afterState;
  const afterBlank = blankStart + parsed.blank.length + 1;

  const cellPositions: number[] = [];
  let cp = afterBlank;
  for (let i = 0; i < parsed.tapeCells.length; i++) {
    cellPositions.push(cp);
    cp += parsed.tapeCells[i].length;
    if (i < parsed.tapeCells.length - 1) cp += 1;
  }

  return (
    <span className="st-tape">
      <CharSpan
        text={parsed.prefix + "#"}
        headPos={headPos}
        startIdx={0}
      />
      <span className="st-section st-rules">
        <span className="st-label">rules</span>
        {parsed.rules.map((rule, i) => {
          const isActive = rule.startsWith("*");
          return (
            <span key={i}>
              {i > 0 && <span className="st-delim">;</span>}
              <span
                className={`st-rule${isActive ? " st-rule-active" : ""}`}
              >
                <CharSpan
                  text={rule}
                  headPos={headPos}
                  startIdx={rulePositions[i]}
                />
              </span>
            </span>
          );
        })}
      </span>
      <CharSpan text="#" headPos={headPos} startIdx={afterRules - 1} />
      <span className="st-section st-accepting">
        <span className="st-label">accept</span>
        <CharSpan
          text={parsed.accepting}
          headPos={headPos}
          startIdx={acceptingStart}
        />
      </span>
      <CharSpan text="#" headPos={headPos} startIdx={afterAccepting - 1} />
      <span className="st-section st-state">
        <span className="st-label">state</span>
        <CharSpan
          text={parsed.state}
          headPos={headPos}
          startIdx={stateStart}
        />
      </span>
      <CharSpan text="#" headPos={headPos} startIdx={afterState - 1} />
      <span className="st-section st-blank">
        <span className="st-label">blank</span>
        <CharSpan
          text={parsed.blank}
          headPos={headPos}
          startIdx={blankStart}
        />
      </span>
      <CharSpan text="#" headPos={headPos} startIdx={afterBlank - 1} />
      <span className="st-section st-tape-cells">
        <span className="st-label">tape</span>
        {parsed.tapeCells.map((cell, i) => {
          const isActive = cell.startsWith("^") || cell.startsWith(">");
          return (
            <span key={i}>
              {i > 0 && <span className="st-delim">,</span>}
              <span
                className={`st-cell${isActive ? " st-cell-active" : ""}`}
              >
                <CharSpan
                  text={cell}
                  headPos={headPos}
                  startIdx={cellPositions[i]}
                />
              </span>
            </span>
          );
        })}
      </span>
    </span>
  );
}

// ── Main component ──

export function TowerView() {
  const [l0, setL0] = useState<L0State | null>(null);
  const [meta, setMeta] = useState<UtmMeta | null>(null);
  const tapeRef = useRef<string>("");
  const unblemishedRef = useRef<string>("");

  useEffect(() => {
    const es = new EventSource("/api/tower");
    es.onmessage = (event) => {
      const msg = JSON.parse(event.data);

      if (msg.type === "total") {
        unblemishedRef.current = msg.unblemished;
        tapeRef.current = msg.tape;

        // Store UTM metadata for client-side decoding
        if (msg.utm_states && msg.utm_symbol_chars) {
          setMeta({
            utmStates: msg.utm_states,
            utmSymbolChars: msg.utm_symbol_chars,
          });
        }

        setL0({
          steps: msg.steps,
          guestSteps: msg.guest_steps,
          stepsPerSec: msg.steps_per_sec,
          state: msg.state,
          headPos: msg.head_pos,
          maxHeadPos: msg.max_head_pos,
          tape: msg.tape,
          tapeLen: msg.tape_len,
        });
      } else if (msg.type === "delta") {
        // Apply overwrites to L0 tape
        const chars = tapeRef.current.split("");
        const end = Math.max(msg.max_head_pos, msg.head_pos) + 10;
        const ub = unblemishedRef.current;
        while (chars.length < end) {
          const pos = chars.length;
          chars.push(pos < ub.length ? ub[pos] : "_");
        }
        for (const [pos, ch] of msg.new_overwrites as [number, string][]) {
          while (chars.length <= pos) chars.push("_");
          chars[pos] = ch;
        }
        tapeRef.current = chars.join("");

        setL0({
          steps: msg.total_steps,
          guestSteps: msg.guest_steps,
          stepsPerSec: msg.steps_per_sec,
          state: msg.state,
          headPos: msg.head_pos,
          maxHeadPos: msg.max_head_pos,
          tape: tapeRef.current,
          tapeLen: msg.tape_len,
        });
      }
    };
    return () => es.close();
  }, []);

  // Build tower by recursively decoding L0
  const tower = useMemo(() => {
    if (!l0) return null;
    return buildTower(l0, meta);
  }, [l0, meta]);

  if (!l0 || !tower) {
    return <div style={{ padding: "16px" }}>Loading...</div>;
  }

  return (
    <div style={{ textAlign: "left", padding: "16px" }}>
      <h2 style={{ marginBottom: "8px" }}>
        Tower &mdash; {l0.steps.toLocaleString()} steps
        {l0.stepsPerSec > 0 && (
          <span
            style={{
              fontWeight: "normal",
              fontSize: "14px",
              marginLeft: "12px",
            }}
          >
            ({l0.stepsPerSec.toFixed(1)}M steps/s,{" "}
            {l0.guestSteps.toLocaleString()} guest steps)
          </span>
        )}
      </h2>
      <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
        {tower.map((level, i) => {
          // For L0, trim to maxHeadPos + 10; for decoded levels show full tape
          const tape =
            i === 0
              ? level.tape.slice(
                  0,
                  Math.max(l0.maxHeadPos, l0.headPos) + 10,
                )
              : level.tape;
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
                  lineHeight: "1.6",
                  overflowWrap: "break-word",
                }}
              >
                <SemanticTape tape={tape} headPos={level.headPos} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
