import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, createSignal, flush, NotReadyError, resolve } from "solid-js";
import {
    createDistance,
    createGeolocation,
    createGeolocationWatcher,
    createWithinRadius,
    makeGeolocation,
    makeGeolocationWatcher,
} from "./geolocation.ts";

/** Make coordinates at a position. */
function coordinates(latitude: number, longitude: number): GeolocationCoordinates {
    const values = {
        latitude,
        longitude,
        altitude: null,
        accuracy: 10,
        altitudeAccuracy: null,
        heading: null,
        speed: null,
    };

    return { ...values, toJSON: () => values };
}

/** Make a position at coordinates. */
function position(coords: GeolocationCoordinates): GeolocationPosition {
    return { coords, timestamp: 0, toJSON: () => ({ coords }) };
}

/** Make a geolocation error. */
function failure(message: string): GeolocationPositionError {
    return { code: 1, message, PERMISSION_DENIED: 1, POSITION_UNAVAILABLE: 2, TIMEOUT: 3 };
}

/** A stand-in for the browser's geolocation that the test answers. */
class StubGeolocation implements Geolocation {
    /** The position the next query answers, or the failure it rejects with. */
    answer: GeolocationPosition | GeolocationPositionError = position(coordinates(48.8566, 2.3522));
    /** The options of each query and watch. */
    readonly options: (PositionOptions | undefined)[] = [];
    /** The watchers by identifier. */
    readonly watchers = new Map<
        number,
        { success: PositionCallback; error: PositionErrorCallback | null | undefined }
    >();

    /** Answer a query with the answer. */
    getCurrentPosition(
        success: PositionCallback,
        error?: PositionErrorCallback | null,
        options?: PositionOptions,
    ): void {
        this.options.push(options);
        if ("coords" in this.answer) {
            success(this.answer);
        } else {
            error?.(this.answer);
        }
    }

    /** Start a watcher, answering it right away. */
    watchPosition(
        success: PositionCallback,
        error?: PositionErrorCallback | null,
        options?: PositionOptions,
    ): number {
        this.options.push(options);
        const identifier = this.watchers.size + 1;
        this.watchers.set(identifier, { success, error });
        this.report(this.answer);

        return identifier;
    }

    /** Stop a watcher. */
    clearWatch(identifier: number): void {
        this.watchers.delete(identifier);
    }

    /** Report a position or failure to every watcher. */
    report(answer: GeolocationPosition | GeolocationPositionError): void {
        for (const watcher of this.watchers.values()) {
            if ("coords" in answer) {
                watcher.success(answer);
            } else {
                watcher.error?.(answer);
            }
        }
    }
}

/** The geolocation of the current test. */
let geolocation = new StubGeolocation();

beforeEach(() => {
    geolocation = new StubGeolocation();
    Object.defineProperty(navigator, "geolocation", { configurable: true, get: () => geolocation });
});

afterEach(() => {
    Reflect.deleteProperty(navigator, "geolocation");
});

test("query the position once, refusing after cleanup and rejecting with the platform's reason", async () => {
    // query, then fail
    const [query, cleanup] = makeGeolocation();
    const located = await query();
    geolocation.answer = failure("permission denied");
    const failed = await query().catch((error: unknown) => error);
    cleanup();
    const stopped = await query().catch((error: unknown) => error);

    expect([located.latitude, failed, stopped]).toEqual([
        48.8566,
        new Error("geolocation failed: permission denied"),
        new DOMException("geolocation query stopped", "AbortError"),
    ]);
});

test("watch the position into a plain object until cleared", () => {
    const [store, cleanup] = makeGeolocationWatcher();
    const located = store.location?.latitude;
    geolocation.report(failure("position unavailable"));
    const failed = [store.location, store.error?.message];
    cleanup();

    expect([located, failed, geolocation.watchers.size]).toEqual([
        48.8566,
        [null, "position unavailable"],
        0,
    ]);
});

test("suspend on the position until it answers, querying again on refetch and changed options", async () => {
    const [options, setOptions] = createSignal<PositionOptions>({ enableHighAccuracy: false });
    const { location, refetch, dispose } = createRoot((disposeRoot) => {
        const [located, locateAgain] = createGeolocation(options);

        return { location: located, refetch: locateAgain, dispose: disposeRoot };
    });
    const latitude = (await resolve(() => location())).latitude;
    refetch();
    flush();
    await resolve(() => location());
    setOptions({ enableHighAccuracy: true });
    flush();
    await resolve(() => location());
    dispose();

    expect({
        latitude,
        queries: geolocation.options.map((used) => used?.enableHighAccuracy),
    }).toEqual({ latitude: 48.8566, queries: [false, false, true] });
});

test("watch the position while enabled, pending until the first fix unless seeded, with its latest error", () => {
    // watch unseeded and seeded, disabled at first
    geolocation.watchers.clear();
    const [isEnabled, setIsEnabled] = createSignal(false, { ownedWrite: true });
    const observed = createRoot((disposeRoot) => {
        const watcher = createGeolocationWatcher(isEnabled);
        const seeded = createGeolocationWatcher(false, undefined, {
            latitude: 51.5074,
            longitude: -0.1278,
        });
        flush();
        const pending = (() => {
            try {
                return watcher.location().latitude;
            } catch (error) {
                return error instanceof NotReadyError ? "pending" : error;
            }
        })();

        // enable, fail, and disable again
        setIsEnabled(true);
        flush();
        const located = watcher.location().latitude;
        geolocation.report(failure("timeout"));
        flush();
        const failed = watcher.error()?.message;
        setIsEnabled(false);
        flush();
        const watching = geolocation.watchers.size;
        disposeRoot();

        return {
            pending,
            seed: [seeded.location().latitude, seeded.location().accuracy],
            located,
            failed,
            watching,
        };
    });

    expect(observed).toEqual({
        pending: "pending",
        seed: [51.5074, Number.POSITIVE_INFINITY],
        located: 48.8566,
        failed: "timeout",
        watching: 0,
    });
});

test("follow the distance to a target and whether it lies within a radius", () => {
    // start in Paris and measure to Paris and London
    geolocation.answer = position(coordinates(48.8566, 2.3522));
    const [target, setTarget] = createSignal(
        { latitude: 48.8566, longitude: 2.3522 },
        { ownedWrite: true },
    );
    const observed = createRoot((disposeRoot) => {
        const kilometres = createDistance(target);
        const metres = createDistance({ latitude: 51.5074, longitude: -0.1278 }, { unit: "m" });
        const near = createWithinRadius({ latitude: 48.8566, longitude: 2.3522 }, 0);
        const far = createWithinRadius({ latitude: 51.5074, longitude: -0.1278 }, 300_000);
        flush();
        const atTarget = kilometres();
        setTarget({ latitude: 51.5074, longitude: -0.1278 });
        flush();
        const toLondon = Math.round(kilometres() ?? 0);
        const values = {
            atTarget,
            toLondon,
            metres: Math.round((metres() ?? 0) / 1000),
            near: near(),
            far: far(),
        };
        disposeRoot();

        return values;
    });

    expect(observed).toEqual({ atTarget: 0, toLondon: 344, metres: 344, near: true, far: false });
});
