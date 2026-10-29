import { expect, test } from "@destack/test";
import { Elevation } from "./elevation.ts";

/** The time the caller authenticated at. */
const AUTHENTICATED_AT = 1_800_000_000_000;

/** An elevation asking for several factors within five minutes. */
const elevation: Elevation = { assurance: 2, maxAge: 5 * 60_000 };

test("meet an elevation until the moment its age ends, the moment until reports", () => {
    // read the elevation just before, at and after its age ends, and with too weak an assurance
    const assurance = { level: 2, authenticatedAt: AUTHENTICATED_AT };
    const ends = AUTHENTICATED_AT + elevation.maxAge;
    const read = (now: number, level = assurance.level) => ({
        meets: Elevation.meets(elevation, { ...assurance, level }, now),
        until: Elevation.until(elevation, { ...assurance, level }, now),
    });

    // end exactly at the moment until reports, so a stream deciding again then sees it lapse
    expect({
        before: read(ends - 1),
        at: read(ends),
        weak: read(AUTHENTICATED_AT, 1),
    }).toEqual({
        before: { meets: true, until: ends },
        at: { meets: false, until: undefined },
        weak: { meets: false, until: undefined },
    });
});
