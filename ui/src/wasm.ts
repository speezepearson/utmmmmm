/**
 * Thin wrapper around the Rust WASM module for UTM encoding/decoding.
 *
 * Usage:
 *   import { wasmEncode, wasmDecode } from "./wasm";
 *   const utmTape = await wasmEncode(specJson, "01011");
 *   const decoded = await wasmDecode(specJson, utmTape);
 */

import init, {
  encode as rawEncode,
  decode as rawDecode,
} from "../pkg/utmmmmm.js";

let ready: Promise<void> | null = null;

function ensureInit(): Promise<void> {
  if (!ready) {
    ready = init().then(() => {});
  }
  return ready;
}

/**
 * Encode a guest TM + tape into a UTM tape string.
 *
 * @param specJson - JSON string of the machine spec (machine-specs.json entry format)
 * @param tapeStr  - tape contents as display characters (e.g. "01011")
 * @returns encoded UTM tape as a string of UTM symbol chars
 */
export async function wasmEncode(
  specJson: string,
  tapeStr: string,
): Promise<string> {
  await ensureInit();
  return rawEncode(specJson, tapeStr);
}

export type DecodedResult = {
  state: string;
  tape: string;
  pos: number;
};

/**
 * Decode a UTM tape back into guest TM state.
 *
 * @param specJson   - JSON string of the guest machine spec
 * @param utmTapeStr - UTM tape as a string of UTM symbol chars
 * @returns decoded guest state
 */
export async function wasmDecode(
  specJson: string,
  utmTapeStr: string,
): Promise<DecodedResult> {
  await ensureInit();
  const json = rawDecode(specJson, utmTapeStr);
  return JSON.parse(json) as DecodedResult;
}
