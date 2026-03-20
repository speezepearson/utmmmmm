import { describe, expect, it } from "vitest";
import { InfiniteUtm } from "./infinite-utm";
import { must } from "./test-util";
import { tapeIndexOf } from "./util";

describe("InfiniteUtm", () => {
  it("has spec=simSpec", () => {
    const utm = new InfiniteUtm();
    expect(utm.spec).toBe(utm.simSpec);
  });

  it("decodes to a machine with the same tape", () => {
    const utm = new InfiniteUtm();
    const dec1 = must(utm.decode());
    const dec2 = must(dec1.decode());
    const headerLen = must(tapeIndexOf(utm.tape, "^"));
    for (let i = 0; i < 2 * headerLen; i++) {
      expect(dec1.tape.get(i), `${i}`).toBe(utm.tape.get(i));
      expect(dec2.tape.get(i)).toBe(dec1.tape.get(i));
    }
  });
});
