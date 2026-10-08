import { expect, test } from "@destack/test";
import { NotReadyError } from "solid-js";
import {
    createDistance,
    createGeolocation,
    createGeolocationWatcher,
    createWithinRadius,
    makeGeolocation,
} from "./geolocation.ts";

/** A seed position in Paris. */
const PARIS = { latitude: 48.8566, longitude: 2.3522 };

/** Read an accessor, naming a pending read. */
function read(accessor: () => { latitude: number }): number | string {
    try {
        return accessor().latitude;
    } catch (error) {
        return error instanceof NotReadyError ? "pending" : String(error);
    }
}

test("stay pending on the server unless seeded, and measure from the seed", async () => {
    const refused = await makeGeolocation()[0]().catch((error: unknown) => error);

    expect({
        refused,
        unseeded: [read(createGeolocation()[0]), read(createGeolocationWatcher(true).location)],
        seeded: [
            read(createGeolocation(undefined, PARIS)[0]),
            read(createGeolocationWatcher(true, undefined, PARIS).location),
        ],
        distance: [createDistance(PARIS)(), createDistance(PARIS, { initialLocation: PARIS })()],
        within: [
            createWithinRadius(PARIS, 10)(),
            createWithinRadius(PARIS, 10, { initialLocation: PARIS })(),
        ],
    }).toEqual({
        refused: new TypeError("geolocation runs only in a browser"),
        unseeded: ["pending", "pending"],
        seeded: [48.8566, 48.8566],
        distance: [null, 0],
        within: [false, true],
    });
});
