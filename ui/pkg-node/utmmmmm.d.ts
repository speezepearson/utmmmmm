/* tslint:disable */
/* eslint-disable */

/**
 * Decode a UTM tape back into guest TM state.
 *
 * - `spec_json`: JSON string of the guest machine spec
 * - `utm_tape_str`: the UTM tape as a string of UTM symbol chars
 *
 * Returns a JSON string: `{ "state": "...", "tape": "...", "pos": N }`
 */
export function decode(spec_json: string, utm_tape_str: string): string;

/**
 * Encode a guest TM into a UTM tape.
 *
 * - `spec_json`: JSON string of the machine spec (same format as machine-specs.json entries)
 * - `tape_str`: the tape contents as display characters (e.g. "01011")
 *
 * Returns the encoded UTM tape as a string of UTM symbol display characters.
 */
export function encode(spec_json: string, tape_str: string): string;
