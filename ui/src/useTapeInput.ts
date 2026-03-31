import { useMemo } from "react";
import { makeInitSnapshot, Symbol, type TuringMachineSpec } from "./types";

export type TapeInputResult = {
  snapshot: import("./types").TuringMachineSnapshot | null;
  invalidChars: string[];
};

export function useTapeInput(
  spec: TuringMachineSpec,
  value: string,
): TapeInputResult {
  const validSymbols = useMemo(() => new Set(spec.allSymbols), [spec]);

  const invalidChars = useMemo(() => {
    const invalid: string[] = [];
    for (const ch of value) {
      if (!validSymbols.has(ch as Symbol) && !invalid.includes(ch)) {
        invalid.push(ch);
      }
    }
    return invalid;
  }, [value, validSymbols]);

  const snapshot = useMemo(() => {
    if (invalidChars.length > 0) return null;
    const tape = [...value].map((c) => Symbol.parse(c));
    return makeInitSnapshot(spec, tape);
  }, [spec, value, invalidChars]);

  return { snapshot, invalidChars };
}
