import { useMemo, useState } from "react";
import "./App.css";
import { TuringMachineViewer } from "./TuringMachineViewer";
import {
  checkPalindromeSpec,
  doubleXSpec,
  flipBitsSpec,
  write1sForeverSpec,
} from "./toy-machines";
import { makeInitSnapshot } from "./types";
import { makeArrayTapeOverlay } from "./util";

export function SamplerPlatter() {
  const initialWrite1sForeverSnapshot = useMemo(
    () => makeInitSnapshot(write1sForeverSpec, makeArrayTapeOverlay([])),
    [],
  );

  const [palindromeInput, setPalindromeInput] = useState("abba");
  const initialPalindromeSnapshot = useMemo(
    () =>
      makeInitSnapshot(
        checkPalindromeSpec,
        makeArrayTapeOverlay(
          palindromeInput
            .split("")
            .filter((c): c is "a" | "b" => c === "a" || c === "b"),
        ),
      ),
    [palindromeInput],
  );

  const [flipBitsInput, setFlipBitsInput] = useState("011");
  const initialFlipBitsSnapshot = useMemo(
    () =>
      makeInitSnapshot(
        flipBitsSpec,
        makeArrayTapeOverlay(
          flipBitsInput
            .split("")
            .filter((c): c is "0" | "1" => c === "0" || c === "1"),
        ),
      ),
    [flipBitsInput],
  );

  const [doubleXCount, setDoubleXCount] = useState(5);
  const initialDoubleXSnapshot = useMemo(
    () =>
      makeInitSnapshot(
        doubleXSpec,
        makeArrayTapeOverlay([
          "$",
          ...Array.from({ length: doubleXCount }, () => "X"),
        ]),
      ),
    [doubleXCount],
  );

  return (
    <div style={{ padding: "24px" }}>
      {/* <h2 style={{ marginTop: "32px" }}>Accept Immediately</h2>
      <TuringMachineViewer init={acceptImmediatelySnapshot} /> */}

      <h2 style={{ marginTop: "32px" }}>Write 1s Forever</h2>
      <TuringMachineViewer init={initialWrite1sForeverSnapshot} />

      <h2 style={{ marginTop: "32px" }}>Flip Bits</h2>
      <label className="tm-tape-input">
        Tape:
        <input
          type="text"
          value={flipBitsInput}
          onChange={(e) => setFlipBitsInput(e.target.value)}
          placeholder="e.g. 011"
          spellCheck={false}
        />
      </label>
      <TuringMachineViewer init={initialFlipBitsSnapshot} />
      <h2>Palindrome Checker</h2>
      <label className="tm-tape-input">
        Tape:
        <input
          type="text"
          value={palindromeInput}
          onChange={(e) => setPalindromeInput(e.target.value)}
          placeholder="e.g. abba"
          spellCheck={false}
        />
      </label>
      <TuringMachineViewer
        key={palindromeInput}
        init={initialPalindromeSnapshot}
      />

      <h2 style={{ marginTop: "32px" }}>Double X</h2>
      <label className="tm-tape-input">
        {" "}
        Number of X's:
        <input
          type="number"
          min={0}
          max={100}
          step={1}
          value={doubleXCount}
          onChange={(e) => setDoubleXCount(Number(e.target.value))}
        />
      </label>
      <TuringMachineViewer key={doubleXCount} init={initialDoubleXSnapshot} />
    </div>
  );
}
