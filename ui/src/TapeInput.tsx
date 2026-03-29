import { useMemo } from "react";
import type { ParsedSpec } from "./parseSpec";
import { useTapeInput } from "./useTapeInput";

type TapeInputProps = {
  parsed: ParsedSpec;
  value: string;
  onChange: (value: string) => void;
};

export function TapeInput({ parsed, value, onChange }: TapeInputProps) {
  const { invalidChars } = useTapeInput(parsed.spec, value);

  const inputSymbolChars = useMemo(() => {
    return Object.entries(parsed.symbolChars).filter(
      ([sym]) => sym !== parsed.blank,
    );
  }, [parsed]);

  return (
    <div>
      <label className="tm-tape-input">
        Input:{" "}
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={`Type using: ${inputSymbolChars.map(([, ch]) => ch).join("")}`}
          spellCheck={false}
        />
      </label>

      {invalidChars.length > 0 && (
        <p style={{ color: "red", margin: "8px 0" }}>
          Invalid character{invalidChars.length > 1 ? "s" : ""}:{" "}
          {invalidChars.map((ch) => (
            <code key={ch} style={{ marginRight: "4px" }}>
              {ch}
            </code>
          ))}
          — allowed:{" "}
          {parsed.spec.allSymbols.map((s) => (
            <code key={s} style={{ marginRight: "4px" }}>
              {s}
            </code>
          ))}
        </p>
      )}
    </div>
  );
}
