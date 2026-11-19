import { expect, test } from "@destack/test";
import { Window } from "../src/preference/index.ts";

/** Saturday 24 October 2026, 12:00 UTC, the day before Vienna leaves summer time at 01:00 UTC. */
const SATURDAY = Date.UTC(2026, 9, 24, 12, 0);

test("end quiet hours in a time zone after the night the clocks go back", () => {
    // end quiet hours from 22:00 to 07:00 after the night the clocks go back, and defer none at noon
    const night = { days: [6], from: "22:00", to: "07:00" };
    const inside = Window.end("Europe/Vienna", [night], Date.UTC(2026, 9, 25, 1, 30));
    const outside = Window.end("Europe/Vienna", [night], SATURDAY);
    expect([inside, outside]).toEqual([Date.UTC(2026, 9, 25, 6, 0), undefined]);
});
