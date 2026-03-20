import { useCallback, useEffect, useRef, useState, useMemo } from "react";
import {
  type TuringMachineSnapshot,
  copySnapshot,
  step,
  getStatus,
} from "./types";
import { MyUtmSnapshot, myUtmSpec } from "./my-utm-spec";

// Section layout: $#RULES#ACCEPT#STATE#BLANK#TAPE
type SectionName = "$" | "rules" | "accept states" | "state" | "blank" | "tape";

const SECTION_COLORS: Record<SectionName, string> = {
  $: "#78716c", // stone
  rules: "#6366f1", // indigo
  "accept states": "#16a34a", // green
  state: "#ea580c", // orange
  blank: "#db2777", // pink
  tape: "#0891b2", // cyan
};

const SECTION_ORDER: SectionName[] = [
  "$",
  "rules",
  "accept states",
  "state",
  "blank",
  "tape",
];

function parseSections(
  tape: readonly string[],
): { name: SectionName; start: number; end: number }[] {
  // Find all # positions
  const hashes: number[] = [];
  for (let i = 0; i < tape.length; i++) {
    if (tape[i] === "#") hashes.push(i);
  }

  const sections: { name: SectionName; start: number; end: number }[] = [];

  // $ is index 0 (if tape starts with $)
  if (tape.length > 0 && tape[0] === "$") {
    sections.push({ name: "$", start: 0, end: 0 });
  }

  // Each # starts the next section (the # itself is a separator, content is after it)
  // But we want to color the # as part of the section it introduces
  for (let i = 0; i < hashes.length && i < 6; i++) {
    const sectionName = SECTION_ORDER[i + 1]; // skip "$"
    const start = hashes[i]; // include the # itself
    const end = i + 1 < hashes.length ? hashes[i + 1] - 1 : tape.length - 1;
    sections.push({ name: sectionName, start, end });
  }

  return sections;
}

function sectionForIndex(
  sections: { name: SectionName; start: number; end: number }[],
  idx: number,
): SectionName | undefined {
  for (const s of sections) {
    if (idx >= s.start && idx <= s.end) return s.name;
  }
  return undefined;
}

// ── Section collapsing ──

type IndexRange = { from: number; to: number }; // inclusive

const SECTION_CAP = 100;
const SECTION_BOOKEND = 20;
const ELLIPSIS_WIDTH = 3;

function collapseSection(
  start: number,
  end: number,
  headPos: number,
): IndexRange[] {
  const len = end - start + 1;
  if (len <= SECTION_CAP) return [{ from: start, to: end }];

  const aTo = start + SECTION_BOOKEND - 1;
  const cFrom = end - SECTION_BOOKEND + 1;

  const fullMiddleBudget = SECTION_CAP - 2 * SECTION_BOOKEND;

  const bMinStart = aTo + 1;
  const bMaxEnd = cFrom - 1;
  const fullBMaxStart = bMaxEnd - fullMiddleBudget + 1;

  let bFrom: number;
  if (headPos < bMinStart) {
    bFrom = bMinStart;
  } else if (headPos > fullBMaxStart + fullMiddleBudget - 1) {
    bFrom = fullBMaxStart;
  } else {
    bFrom = Math.max(
      bMinStart,
      Math.min(fullBMaxStart, headPos - Math.floor(fullMiddleBudget / 2)),
    );
  }

  const adjacentToPrefix = bFrom === bMinStart;
  const adjacentToSuffix = bFrom + fullMiddleBudget - 1 === bMaxEnd;

  if (adjacentToPrefix || adjacentToSuffix) {
    return [
      { from: start, to: aTo },
      { from: bFrom, to: bFrom + fullMiddleBudget - 1 },
      { from: cFrom, to: end },
    ];
  }

  const shrunkBudget = fullMiddleBudget - ELLIPSIS_WIDTH;
  const shrunkBMaxStart = bMaxEnd - shrunkBudget + 1;
  bFrom = Math.max(
    bMinStart,
    Math.min(shrunkBMaxStart, headPos - Math.floor(shrunkBudget / 2)),
  );

  return [
    { from: start, to: aTo },
    { from: bFrom, to: bFrom + shrunkBudget - 1 },
    { from: cFrom, to: end },
  ];
}

function computeVisibleRanges(
  tapeLen: number,
  headPos: number,
  sections: { name: SectionName; start: number; end: number }[] | undefined,
  visibleRadius: number | undefined,
): IndexRange[] {
  let ranges: IndexRange[];

  if (sections && sections.length > 0) {
    ranges = [];
    for (const s of sections) {
      ranges.push(...collapseSection(s.start, s.end, headPos));
    }
  } else {
    ranges = [{ from: 0, to: tapeLen - 1 }];
  }

  if (visibleRadius != null) {
    const gFrom = Math.max(0, headPos - visibleRadius);
    const gTo = Math.min(tapeLen - 1, headPos + visibleRadius);
    ranges = ranges
      .map((r) => ({ from: Math.max(r.from, gFrom), to: Math.min(r.to, gTo) }))
      .filter((r) => r.from <= r.to);
  }

  if (ranges.length === 0) return [];
  ranges.sort((a, b) => a.from - b.from);
  const merged: IndexRange[] = [{ ...ranges[0] }];
  for (let i = 1; i < ranges.length; i++) {
    const last = merged[merged.length - 1];
    if (ranges[i].from <= last.to + 1) {
      last.to = Math.max(last.to, ranges[i].to);
    } else {
      merged.push({ ...ranges[i] });
    }
  }
  return merged;
}

// ── Chunk-aware parsing for rules and tape ──

/** A chunk is a logical unit (a rule or a tape cell) within a section. */
type Chunk = { start: number; end: number }; // inclusive tape indices

/** Parse rule boundaries within the rules section.
 *  Each rule starts with '.' or '*' and ends just before the next '.'/'*' or at section end. */
function parseRuleChunks(
  tape: readonly string[],
  sectionStart: number,
  sectionEnd: number,
): Chunk[] {
  const chunks: Chunk[] = [];
  // sectionStart is the '#' — content starts at sectionStart + 1
  let i = sectionStart + 1;
  while (i <= sectionEnd) {
    const sym = tape[i];
    if (sym === "." || sym === "*") {
      const chunkStart = i;
      i++;
      // Scan to end of this rule: next '.' or '*' (skip the ';' separator before it)
      while (i <= sectionEnd && tape[i] !== "." && tape[i] !== "*") {
        i++;
      }
      // The chunk ends at i-1 (which is ';' or '#' or the last char)
      chunks.push({ start: chunkStart, end: i - 1 });
    } else {
      i++;
    }
  }
  return chunks;
}

/** Parse tape cell boundaries within the tape section.
 *  Each cell starts with ',' or '^' or '>' followed by fixed-width binary bits. */
function parseTapeCellChunks(
  tape: readonly string[],
  sectionStart: number,
  sectionEnd: number,
): Chunk[] {
  const chunks: Chunk[] = [];
  let i = sectionStart + 1; // skip '#'
  while (i <= sectionEnd) {
    const sym = tape[i];
    if (sym === "," || sym === "^" || sym === ">") {
      const chunkStart = i;
      i++;
      while (
        i <= sectionEnd &&
        tape[i] !== "," &&
        tape[i] !== "^" &&
        tape[i] !== ">"
      ) {
        i++;
      }
      chunks.push({ start: chunkStart, end: i - 1 });
    } else {
      i++;
    }
  }
  return chunks;
}

/** Pick which chunks to show: first N, last N, and a window around the chunk containing headPos. */
function selectChunks(
  chunks: Chunk[],
  headPos: number,
  firstCount: number,
  lastCount: number,
  windowRadius: number,
): Chunk[] {
  if (chunks.length <= firstCount + lastCount) return chunks;

  // Find the chunk the head is on
  let headChunkIdx = -1;
  for (let i = 0; i < chunks.length; i++) {
    if (headPos >= chunks[i].start && headPos <= chunks[i].end) {
      headChunkIdx = i;
      break;
    }
  }

  const selected = new Set<number>();

  // First N
  for (let i = 0; i < Math.min(firstCount, chunks.length); i++) {
    selected.add(i);
  }
  // Last N
  for (let i = Math.max(0, chunks.length - lastCount); i < chunks.length; i++) {
    selected.add(i);
  }
  // Window around head
  if (headChunkIdx >= 0) {
    for (
      let i = Math.max(0, headChunkIdx - windowRadius);
      i <= Math.min(chunks.length - 1, headChunkIdx + windowRadius);
      i++
    ) {
      selected.add(i);
    }
  }

  return Array.from(selected)
    .sort((a, b) => a - b)
    .map((i) => chunks[i]);
}

/** Convert selected chunks to IndexRanges, inserting gaps where chunks are non-contiguous. */
function chunksToRanges(selectedChunks: Chunk[]): IndexRange[] {
  if (selectedChunks.length === 0) return [];
  // Each chunk becomes its own range; merging will combine contiguous ones
  const ranges: IndexRange[] = selectedChunks.map((c) => ({
    from: c.start,
    to: c.end,
  }));
  // Merge adjacent
  const merged: IndexRange[] = [{ ...ranges[0] }];
  for (let i = 1; i < ranges.length; i++) {
    const last = merged[merged.length - 1];
    if (ranges[i].from <= last.to + 2) {
      // +2 to merge chunks separated by just a ';'
      last.to = Math.max(last.to, ranges[i].to);
    } else {
      merged.push({ ...ranges[i] });
    }
  }
  return merged;
}

/** Compute visible ranges with chunk-aware collapsing for rules and tape sections. */
function computeChunkAwareRanges(
  tape: readonly string[],
  headPos: number,
  sections: { name: SectionName; start: number; end: number }[],
  visibleRadius: number | undefined,
): IndexRange[] {
  let ranges: IndexRange[] = [];

  for (const s of sections) {
    if (s.name === "rules") {
      const ruleChunks = parseRuleChunks(tape, s.start, s.end);
      const selected = selectChunks(ruleChunks, headPos, 3, 3, 1);
      if (selected.length === 0) {
        // Show '#' at minimum
        ranges.push({ from: s.start, to: s.start });
      } else {
        // Include the '#' at section start
        const chunkRanges = chunksToRanges(selected);
        chunkRanges[0].from = Math.min(chunkRanges[0].from, s.start);
        ranges.push(...chunkRanges);
      }
    } else if (s.name === "tape") {
      const cellChunks = parseTapeCellChunks(tape, s.start, s.end);
      const selected = selectChunks(cellChunks, headPos, 10, 10, 10);
      if (selected.length === 0) {
        ranges.push({ from: s.start, to: s.start });
      } else {
        const chunkRanges = chunksToRanges(selected);
        chunkRanges[0].from = Math.min(chunkRanges[0].from, s.start);
        ranges.push(...chunkRanges);
      }
    } else {
      // For small sections ($, accept states, state, blank), show everything
      ranges.push(...collapseSection(s.start, s.end, headPos));
    }
  }

  // Intersect with global radius
  if (visibleRadius != null) {
    const gFrom = Math.max(0, headPos - visibleRadius);
    const gTo = Math.min(tape.length - 1, headPos + visibleRadius);
    ranges = ranges
      .map((r) => ({
        from: Math.max(r.from, gFrom),
        to: Math.min(r.to, gTo),
      }))
      .filter((r) => r.from <= r.to);
  }

  // Merge adjacent/overlapping
  if (ranges.length === 0) return [];
  ranges.sort((a, b) => a.from - b.from);
  const merged: IndexRange[] = [{ ...ranges[0] }];
  for (let i = 1; i < ranges.length; i++) {
    const last = merged[merged.length - 1];
    if (ranges[i].from <= last.to + 1) {
      last.to = Math.max(last.to, ranges[i].to);
    } else {
      merged.push({ ...ranges[i] });
    }
  }
  return merged;
}

/** Render a single UTM rule as a grouped inline element. */
function MyUTMRule({
  tape,
  chunk,
  headPos,
  color,
}: {
  tape: readonly string[];
  chunk: Chunk;
  headPos: number;
  color: string;
}) {
  const isActive = tape[chunk.start] === "*";
  const elements: React.ReactNode[] = [];
  for (let i = chunk.start; i <= chunk.end; i++) {
    const isHead = i === headPos;
    elements.push(
      <span
        key={i}
        className={isHead ? "utm-cell utm-cell-head" : "utm-cell"}
        style={isHead ? undefined : { color }}
      >
        {tape[i]}
      </span>,
    );
  }
  return (
    <span className={`utm-rule${isActive ? " utm-rule-active" : ""}`}>
      {elements}
    </span>
  );
}

/** Render a single simulated tape cell as a grouped inline element. */
function MyUtmCell({
  tape,
  chunk,
  headPos,
  color,
}: {
  tape: readonly string[];
  chunk: Chunk;
  headPos: number;
  color: string;
}) {
  const marker = tape[chunk.start];
  const isSimHead = marker === "^" || marker === ">";
  const elements: React.ReactNode[] = [];
  for (let i = chunk.start; i <= chunk.end; i++) {
    const isHead = i === headPos;
    elements.push(
      <span
        key={i}
        className={isHead ? "utm-cell utm-cell-head" : "utm-cell"}
        style={isHead ? undefined : { color }}
      >
        {tape[i]}
      </span>,
    );
  }
  return (
    <span className={`utm-tcell${isSimHead ? " utm-tcell-head" : ""}`}>
      {elements}
    </span>
  );
}

/** Render a tape as a wrapping grid of colored characters with head highlight.
 *  Rules and tape sections use chunk-aware collapsing with MyUTMRule/MyUtmCell components. */
function TapeDisplay({
  tape,
  headPos,
  sections,
  label,
  stateLabel,
  visibleRadius,
}: {
  tape: readonly string[];
  headPos: number;
  sections?: { name: SectionName; start: number; end: number }[];
  label: string;
  stateLabel?: string;
  visibleRadius?: number;
}) {
  // Parse chunks for rules and tape sections
  const ruleChunks = useMemo(() => {
    if (!sections) return [];
    const rulesSection = sections.find((s) => s.name === "rules");
    if (!rulesSection) return [];
    return parseRuleChunks(tape, rulesSection.start, rulesSection.end);
  }, [tape, sections]);

  const tapeCellChunks = useMemo(() => {
    if (!sections) return [];
    const tapeSection = sections.find((s) => s.name === "tape");
    if (!tapeSection) return [];
    return parseTapeCellChunks(tape, tapeSection.start, tapeSection.end);
  }, [tape, sections]);

  // Build a map from tape index → chunk (for rules and tape cells)
  const chunkMap = useMemo(() => {
    const map = new Map<number, { chunk: Chunk; type: "rule" | "cell" }>();
    for (const c of ruleChunks) {
      map.set(c.start, { chunk: c, type: "rule" });
    }
    for (const c of tapeCellChunks) {
      map.set(c.start, { chunk: c, type: "cell" });
    }
    return map;
  }, [ruleChunks, tapeCellChunks]);

  // Build set of indices that are inside a chunk but not the start (to skip them)
  const insideChunk = useMemo(() => {
    const set = new Set<number>();
    for (const c of ruleChunks) {
      for (let i = c.start + 1; i <= c.end; i++) set.add(i);
    }
    for (const c of tapeCellChunks) {
      for (let i = c.start + 1; i <= c.end; i++) set.add(i);
    }
    return set;
  }, [ruleChunks, tapeCellChunks]);

  const visibleRanges = useMemo(() => {
    if (sections && sections.length > 0) {
      return computeChunkAwareRanges(tape, headPos, sections, visibleRadius);
    }
    return computeVisibleRanges(tape.length, headPos, sections, visibleRadius);
  }, [tape, headPos, sections, visibleRadius]);

  // Build section label positions: label at first visible index of each section
  const sectionLabelAt = useMemo(() => {
    const map = new Map<number, SectionName>();
    if (!sections) return map;
    for (const s of sections) {
      if (s.name === "$") continue;
      for (const r of visibleRanges) {
        if (r.to < s.start || r.from > s.end) continue;
        const firstVisible = Math.max(r.from, s.start);
        map.set(firstVisible, s.name);
        break;
      }
    }
    return map;
  }, [sections, visibleRanges]);

  const elements: React.ReactNode[] = [];

  for (let ri = 0; ri < visibleRanges.length; ri++) {
    const range = visibleRanges[ri];

    // Leading ellipsis
    if (ri === 0 && range.from > 0) {
      elements.push(
        <span key="pre-ellipsis" className="utm-ellipsis">
          ...
        </span>,
      );
    } else if (ri > 0) {
      elements.push(
        <span key={`gap-${ri}`} className="utm-ellipsis">
          ...
        </span>,
      );
    }

    for (let i = range.from; i <= range.to; i++) {
      // Skip indices inside a chunk (they're rendered as part of the chunk component)
      if (insideChunk.has(i)) continue;

      const section = sections ? sectionForIndex(sections, i) : undefined;
      const color = section ? SECTION_COLORS[section] : "var(--text)";
      const sLabel = sectionLabelAt.get(i);

      const chunkInfo = chunkMap.get(i);
      if (chunkInfo && chunkInfo.chunk.end <= range.to) {
        // Render as a chunk component
        const chunkEl =
          chunkInfo.type === "rule" ? (
            <MyUTMRule
              key={`rule-${i}`}
              tape={tape}
              chunk={chunkInfo.chunk}
              headPos={headPos}
              color={color}
            />
          ) : (
            <MyUtmCell
              key={`cell-${i}`}
              tape={tape}
              chunk={chunkInfo.chunk}
              headPos={headPos}
              color={color}
            />
          );

        if (sLabel) {
          elements.push(
            <span
              key={`label-${i}`}
              style={{ position: "relative", display: "inline" }}
            >
              <span className="utm-section-label" style={{ color }}>
                {sLabel}
              </span>
              {chunkEl}
            </span>,
          );
        } else {
          elements.push(chunkEl);
        }
      } else {
        // Render individual character (for '#', '$', non-chunk content)
        const isHead = i === headPos;
        elements.push(
          <span key={i} style={{ position: "relative", display: "inline" }}>
            {sLabel && (
              <span className="utm-section-label" style={{ color }}>
                {sLabel}
              </span>
            )}
            <span
              className={isHead ? "utm-cell utm-cell-head" : "utm-cell"}
              style={isHead ? undefined : { color }}
            >
              {tape[i]}
            </span>
          </span>,
        );
      }
    }
  }

  // Trailing ellipsis
  if (
    visibleRanges.length > 0 &&
    visibleRanges[visibleRanges.length - 1].to < tape.length - 1
  ) {
    elements.push(
      <span key="post-ellipsis" className="utm-ellipsis">
        ...
      </span>,
    );
  }

  return (
    <div style={{ marginBottom: 12 }}>
      <div className="utm-tape-label">
        {label}
        {stateLabel && (
          <span style={{ color: "var(--text)" }}>{stateLabel}</span>
        )}
      </div>
      <div className="utm-tape-wrap">{elements}</div>
    </div>
  );
}

type MyUTMViewerProps<SimState extends string, SimSymbol extends string> = {
  initialSim: TuringMachineSnapshot<SimState, SimSymbol>;
};

export function MyUTMViewer<SimState extends string, SimSymbol extends string>({
  initialSim,
}: MyUTMViewerProps<SimState, SimSymbol>) {
  const makeInitial = useCallback(() => {
    const utmSnapshot = myUtmSpec.encode(initialSim) as MyUtmSnapshot<
      SimState,
      SimSymbol
    >;
    if (!(utmSnapshot instanceof MyUtmSnapshot)) {
      throw new Error("utmSnapshot is not a MyUtmSnapshot???");
    }
    const decoded = utmSnapshot.decode();
    return { utmSnapshot, decoded };
  }, [initialSim]);

  const [utmSnapshot, setUtmSnapshot] = useState(
    () => makeInitial().utmSnapshot,
  );
  const [utmStatus, setUtmStatus] = useState<"accept" | "reject" | "running">(
    "running",
  );
  const [lastDecoded, setLastDecoded] = useState<TuringMachineSnapshot<
    SimState,
    SimSymbol
  > | null>(() => makeInitial().decoded ?? null);
  const [playing, setPlaying] = useState(false);
  const [logFps, setLogFps] = useState(Math.log10(5));
  const fps = Math.round(10 ** logFps);
  const [logRadius, setLogRadius] = useState(5);
  const visibleRadius = Math.round(10 ** logRadius);
  const [stepCount, setStepCount] = useState(0);

  const utmRef = useRef(utmSnapshot);
  const statusRef = useRef(utmStatus);
  const lastDecodedRef = useRef(lastDecoded);
  const stepCountRef = useRef(0);

  useEffect(() => {
    utmRef.current = utmSnapshot;
  }, [utmSnapshot]);
  useEffect(() => {
    statusRef.current = utmStatus;
  }, [utmStatus]);
  useEffect(() => {
    lastDecodedRef.current = lastDecoded;
  }, [lastDecoded]);
  useEffect(() => {
    stepCountRef.current = stepCount;
  }, [stepCount]);

  const MAX_HISTORY = 20;
  const historyRef = useRef<
    {
      snap: MyUtmSnapshot<SimState, SimSymbol>;
      decoded: TuringMachineSnapshot<SimState, SimSymbol> | null;
      stepCount: number;
    }[]
  >([]);
  const [canRewind, setCanRewind] = useState(false);

  const pushHistory = useCallback(() => {
    const h = historyRef.current;
    h.push({
      snap: new MyUtmSnapshot(utmRef.current),
      decoded: lastDecodedRef.current
        ? copySnapshot(lastDecodedRef.current)
        : null,
      stepCount: stepCountRef.current,
    });
    if (h.length > MAX_HISTORY) h.shift();
    setCanRewind(true);
  }, []);

  const doStep = useCallback(() => {
    if (statusRef.current !== "running") return;
    pushHistory();
    const next = utmRef.current;
    const st = getStatus(step(next));

    utmRef.current = next;
    statusRef.current = st;
    setUtmSnapshot(next);
    setUtmStatus(st);
    setStepCount((c) => c + 1);

    const decoded = next.decode();
    if (decoded) {
      lastDecodedRef.current = decoded;
      setLastDecoded(decoded);
    }

    if (st !== "running") {
      setPlaying(false);
    }
  }, [pushHistory]);

  const doStepState = useCallback(() => {
    if (statusRef.current !== "running") return;
    pushHistory();
    const snap = new MyUtmSnapshot(utmRef.current);
    const startState = snap.state;
    let st: "accept" | "reject" | "running" = "running";
    let steps = 0;
    while (st === "running" && snap.state === startState) {
      st = getStatus(step(snap));
      steps++;
    }

    utmRef.current = snap;
    statusRef.current = st;
    setUtmSnapshot(snap);
    setUtmStatus(st);
    setStepCount((c) => c + steps);

    const decoded = snap.decode();
    if (decoded) {
      lastDecodedRef.current = decoded;
      setLastDecoded(decoded);
    }

    if (st !== "running") {
      setPlaying(false);
    }
  }, [pushHistory]);

  const reset = useCallback(() => {
    const { utmSnapshot: snap, decoded } = makeInitial();
    utmRef.current = snap;
    statusRef.current = "running";
    lastDecodedRef.current = decoded ?? null;
    setUtmSnapshot(snap);
    setUtmStatus("running");
    setLastDecoded(decoded ?? null);
    setPlaying(false);
    setStepCount(0);
    historyRef.current = [];
    setCanRewind(false);
  }, [makeInitial]);

  const rewind = useCallback(() => {
    const h = historyRef.current;
    const entry = h.pop();
    if (!entry) return;
    utmRef.current = entry.snap;
    statusRef.current = "running";
    lastDecodedRef.current = entry.decoded;
    setUtmSnapshot(entry.snap);
    setUtmStatus("running");
    setLastDecoded(entry.decoded);
    setStepCount(entry.stepCount);
    setPlaying(false);
    setCanRewind(h.length > 0);
  }, []);

  const fpsRef = useRef(fps);
  useEffect(() => {
    fpsRef.current = fps;
  }, [fps]);
  const accumRef = useRef(0);

  useEffect(() => {
    if (!playing) {
      accumRef.current = 0;
      return;
    }
    const MAX_RENDER_FPS = 30;
    const interval = setInterval(() => {
      if (statusRef.current !== "running") return;
      accumRef.current += fpsRef.current / MAX_RENDER_FPS;
      const stepsThisFrame = Math.floor(accumRef.current);
      accumRef.current -= stepsThisFrame;
      if (stepsThisFrame === 0) return;

      pushHistory();
      const snap = new MyUtmSnapshot(utmRef.current);
      let st: "accept" | "reject" | "running" = "running";
      for (let i = 0; i < stepsThisFrame; i++) {
        st = getStatus(step(snap));
        if (st !== "running") break;
      }

      utmRef.current = snap;
      statusRef.current = st;
      stepCountRef.current += stepsThisFrame;
      setUtmSnapshot(snap);
      setUtmStatus(st);
      setStepCount(stepCountRef.current);

      // Only decode once per render frame (on the final state)
      const decoded = snap.decode();
      if (decoded) {
        lastDecodedRef.current = decoded;
        setLastDecoded(decoded);
      }

      if (st !== "running") {
        setPlaying(false);
      }
    }, 1000 / MAX_RENDER_FPS);
    return () => clearInterval(interval);
  }, [playing, pushHistory]);

  const halted = utmStatus !== "running";

  const utmSections = useMemo(
    () => parseSections(utmSnapshot.tape as string[]),
    [utmSnapshot.tape],
  );

  // Pad simulated tape for display
  const simTapeDisplay = useMemo(() => {
    if (!lastDecoded) return null;
    const tape = lastDecoded.tape.slice() as string[];
    // while (tape.length <= lastDecoded.pos) {
    //   tape.push(initialSim.spec.blank);
    // }
    return tape;
  }, [lastDecoded]);

  return (
    <div className="tm-viewer">
      <div className="utm-status-line">
        UTM step: {stepCount} | UTM state: {utmSnapshot.state} | head:{" "}
        {utmSnapshot.pos}
      </div>

      <TapeDisplay
        tape={utmSnapshot.tape as string[]}
        headPos={utmSnapshot.pos}
        sections={utmSections}
        label="UTM tape"
        visibleRadius={visibleRadius}
      />

      {lastDecoded && simTapeDisplay && (
        <TapeDisplay
          tape={simTapeDisplay}
          headPos={lastDecoded.pos}
          label="Simulated TM tape"
          stateLabel={` (state=${lastDecoded.state})`}
        />
      )}

      {halted && (
        <div className={`tm-result tm-result-${utmStatus}`}>
          UTM {utmStatus.toUpperCase()}
        </div>
      )}

      <div className="tm-controls">
        <button onClick={doStep} disabled={halted}>
          Step
        </button>
        <button onClick={doStepState} disabled={halted}>
          Step State
        </button>
        <button onClick={() => setPlaying((p) => !p)} disabled={halted}>
          {playing ? "Pause" : "Play"}
        </button>
        <button onClick={rewind} disabled={!canRewind}>
          Rewind
        </button>
        <button onClick={reset}>Reset</button>
        <label className="tm-fps">
          FPS:
          <input
            type="range"
            min={0}
            max={6}
            step={0.1}
            value={logFps}
            onChange={(e) => setLogFps(Number(e.target.value))}
          />
          <span>{fps}</span>
        </label>
        <label className="tm-fps">
          Radius:
          <input
            type="range"
            min={0}
            max={3}
            step={0.1}
            value={logRadius}
            onChange={(e) => setLogRadius(Number(e.target.value))}
          />
          <span>{visibleRadius}</span>
        </label>
      </div>
    </div>
  );
}
