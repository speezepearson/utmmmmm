import { useMemo } from "react";
import type { ParsedSpec } from "./parseSpec";
import { type Symbol, type TuringMachineSnapshot, makeInitSnapshot } from "./types";

/**
 * Convert a user-typed string into an array of Symbols using the spec's symbolChars mapping.
 * Each character in `input` is looked up in the reverse of symbolChars (displayChar -> Symbol).
 */
function parseInput(parsed: ParsedSpec, input: string): Symbol[] | null {
  const charToSymbol: Record<string, Symbol> = {};
  for (const [name, displayChar] of Object.entries(parsed.symbolChars)) {
    // name is the rust-side symbol name, displayChar is what the user types
    void name;
    charToSymbol[displayChar] = displayChar;
  }

  const tape: Symbol[] = [];
  for (const ch of input) {
    const sym = charToSymbol[ch];
    if (sym === undefined) return null;
    tape.push(sym);
  }
  return tape;
}

export function useTapeInput(
  spec: ParsedSpec["spec"],
  input: string,
): { snapshot: TuringMachineSnapshot | null } {
  const snapshot = useMemo(() => {
    // Find the parsed spec for this spec (we need symbolChars)
    // For now, build a simple char->symbol mapping from spec.allSymbols
    // where each symbol IS the display character
    const tape: Symbol[] = [];
    const validSymbols = new Set(spec.allSymbols);
    for (const ch of input) {
      const sym = ch as Symbol;
      if (!validSymbols.has(sym)) return null;
      tape.push(sym);
    }
    if (tape.length === 0) {
      return makeInitSnapshot(spec, [spec.blank]);
    }
    return makeInitSnapshot(spec, tape);
  }, [spec, input]);

  return { snapshot };
}

export function TapeInput({
  parsed,
  value,
  onChange,
}: {
  parsed: ParsedSpec;
  value: string;
  onChange: (value: string) => void;
}) {
  void parseInput; // available for future use

  const validChars = Object.values(parsed.symbolChars).filter(
    (ch) => ch !== parsed.symbolChars[parsed.blank],
  );

  return (
    <div style={{ margin: "12px 0" }}>
      <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>
        Tape input ({validChars.join(", ")}):
      </label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        style={{
          fontFamily: "var(--mono)",
          fontSize: "16px",
          padding: "6px 10px",
          border: "1px solid var(--border)",
          borderRadius: "4px",
          width: "100%",
          boxSizing: "border-box",
        }}
        placeholder={`e.g. ${validChars.slice(0, 5).join("")}`}
      />
    </div>
  );
}
