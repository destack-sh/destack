import { describe, expect, test } from "bun:test";
import { timedeltaFromISOFormat, timedeltaToISOFormat } from "@destack/utils/time";

describe("timedeltaFromISOFormat", () => {
  test.each([
    ["P0D"],
    ["P1D"],
    ["PT1H"],
    ["PT1M"],
    ["PT1S"],
    ["P1W"],
    ["P1W1D"],
    ["P2W"],
    ["-PT1H"],
    ["P1DT12H"],
    ["PT12H30M"],
    ["P1DT12H30M15S"],
    ["-PT12H30M15S"],
    ["PT1H1M1.123456666S"],
    ["PT12.345678123S"],
  ])("timedeltaFromISOFormat(%s)", (iso) => {
    const parsed = timedeltaFromISOFormat(iso);
    const rendered = timedeltaToISOFormat(parsed);
    expect(iso).toBe(rendered);

    const roundtrip = timedeltaFromISOFormat(rendered);
    expect(parsed).toEqual(roundtrip);
    expect(timedeltaToISOFormat(parsed)).toBe(rendered);
  });
});
