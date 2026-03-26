import { useState } from "react";
import { TuringMachineViewer } from "./TuringMachineViewer";
import { machineSpecs } from "./parseSpec";
import { TapeInput, useTapeInput } from "./TapeInput";

const STORAGE_KEY = "welcomeModalDismissed";

function getPalindromeSpec() {
  const spec = machineSpecs.find((s) => s.name === "Check Palindrome");
  if (!spec) throw new Error("Check Palindrome spec not found");
  return spec;
}
const palindromeSpec = getPalindromeSpec();

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
