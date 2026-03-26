import { useEffect, useState } from "react";
import { TuringMachineViewer } from "./TuringMachineViewer";
import { machineSpecs } from "./parseSpec";
import { TapeInput, useTapeInput } from "./TapeInput";
import { wasmEncode, wasmDecode } from "./wasm";
import rawSpecs from "./machine-specs.json";

const STORAGE_KEY = "welcomeModalDismissed";

function getPalindromeSpec() {
  const spec = machineSpecs.find((s) => s.name === "Check Palindrome");
  if (!spec) throw new Error("Check Palindrome spec not found");
  return spec;
}
const palindromeSpec = getPalindromeSpec();

function getFlipBitsRawJson(): string {
  const raw = (rawSpecs as Array<Record<string, unknown>>).find(
    (s) => s.name === "Flip Bits",
  );
  if (!raw) throw new Error("Flip Bits spec not found in raw specs");
  return JSON.stringify(raw);
}
const flipBitsJson = getFlipBitsRawJson();

function EncodingDemo() {
  const [input, setInput] = useState("01011");
  const [encoded, setEncoded] = useState<string | null>(null);
  const [decoded, setDecoded] = useState<{
    state: string;
    tape: string;
    pos: number;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setError(null);
    setEncoded(null);
    setDecoded(null);

    if (!input) return;

    wasmEncode(flipBitsJson, input)
      .then((enc) => {
        if (cancelled) return;
        setEncoded(enc);
        return wasmDecode(flipBitsJson, enc);
      })
      .then((dec) => {
        if (cancelled || !dec) return;
        setDecoded(dec);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(String(e));
      });

    return () => {
      cancelled = true;
    };
  }, [input]);

  return (
    <div
      style={{
        marginTop: "16px",
        padding: "12px",
        background: "rgba(128,128,128,0.1)",
        borderRadius: "8px",
        textAlign: "left",
        fontSize: "13px",
      }}
    >
      <strong>UTM Encoding (via Rust/WASM):</strong>
      <div style={{ marginTop: "8px" }}>
        <label>
          Flip Bits tape:{" "}
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            style={{
              fontFamily: "var(--mono)",
              fontSize: "14px",
              padding: "4px 8px",
              border: "1px solid var(--border)",
              borderRadius: "4px",
              width: "120px",
            }}
            placeholder="e.g. 01011"
          />
        </label>
      </div>
      {error && (
        <div style={{ color: "red", marginTop: "4px" }}>{error}</div>
      )}
      {encoded && (
        <div style={{ marginTop: "8px" }}>
          <div>
            Encoded UTM tape ({encoded.length} symbols):
          </div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: "11px",
              wordBreak: "break-all",
              maxHeight: "60px",
              overflow: "auto",
              background: "rgba(0,0,0,0.05)",
              padding: "4px",
              borderRadius: "4px",
              marginTop: "4px",
            }}
          >
            {encoded}
          </div>
        </div>
      )}
      {decoded && (
        <div style={{ marginTop: "8px" }}>
          Decoded back: state=<code>{decoded.state}</code>, tape=
          <code>{decoded.tape}</code>, pos={decoded.pos}
        </div>
      )}
    </div>
  );
}

export function WelcomeModal() {
  const [visible, setVisible] = useState(
    true // () => !localStorage.getItem(STORAGE_KEY),
  );

  const [palindromeInput, setPalindromeInput] = useState("racecar");

  const { snapshot } = useTapeInput(palindromeSpec.spec, palindromeInput);


  if (!visible) return null;

  const dismiss = () => {
    localStorage.setItem(STORAGE_KEY, "1");
    setVisible(false);
  };

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0, 0, 0, 0.5)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
      onClick={dismiss}
    >
      <div
        style={{
          background: "var(--bg)",
          border: "1px solid var(--border)",
          borderRadius: "12px",
          padding: "32px",
          maxWidth: "480px",
          width: "90%",
          boxShadow: "var(--shadow)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 style={{ marginTop: 0 }}>Welcome to the Self-Simulating Tower!</h2>

        <p style={{ textAlign: 'left', marginBottom: "16px", lineHeight: "1.6" }}>
          Here's a Turing machine (you know what a <a href="https://en.wikipedia.org/wiki/Turing_machine">Turing machine</a> is, right?):
        </p>

        {snapshot && <TuringMachineViewer key={palindromeInput} init={snapshot} />}

        <TapeInput parsed={palindromeSpec} value={palindromeInput} onChange={setPalindromeInput} />

        <EncodingDemo />

        <button
          onClick={dismiss}
          style={{
            marginTop: "16px",
            fontFamily: "var(--mono)",
            fontSize: "14px",
            padding: "8px 20px",
            borderRadius: "6px",
            border: "1px solid var(--border)",
            background: "var(--accent)",
            color: "#fff",
            cursor: "pointer",
          }}
        >
          Close
        </button>
      </div>
    </div>
  );
}
