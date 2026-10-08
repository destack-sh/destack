import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    NotReadyError,
    onCleanup,
} from "solid-js";
import { access, type MaybeAccessor, TRANSPARENT } from "./utils.ts";

/** A position on Earth in degrees. */
export type GeolocationCoord = { readonly latitude: number; readonly longitude: number };

/** The radius of Earth in kilometres, which the haversine distance scales by. */
const EARTH_RADIUS = 6371;

/** The metres in a kilometre. */
const KILOMETRE = 1000;

/** The position options every query starts from: coarse, fresh, and waiting as long as it takes. */
const DEFAULT_OPTIONS: PositionOptions = {
    enableHighAccuracy: false,
    maximumAge: 0,
    timeout: Number.POSITIVE_INFINITY,
};

/** Make a function that queries the position once, and one that stops later queries, rejecting where the device cannot locate. */
export function makeGeolocation(
    options?: PositionOptions,
): [query: () => Promise<GeolocationCoordinates>, cleanup: () => void] {
    // query nothing on the server
    if (isServer) {
        return [
            async () => Promise.reject(new TypeError("geolocation runs only in a browser")),
            () => {},
        ];
    }

    // query while active, refusing once stopped
    let isActive = true;
    const query = async (): Promise<GeolocationCoordinates> => {
        if (!isActive) {
            throw new DOMException("geolocation query stopped", "AbortError");
        }

        return currentPosition(options);
    };

    return [
        query,
        () => {
            isActive = false;
        },
    ];
}

/** Watch the position into a plain object, without an owner, until the returned function is called. */
export function makeGeolocationWatcher(
    options?: PositionOptions,
): [
    store: { location: GeolocationCoordinates | null; error: GeolocationPositionError | null },
    cleanup: () => void,
] {
    // watch nothing on the server
    const store: {
        location: GeolocationCoordinates | null;
        error: GeolocationPositionError | null;
    } = { location: null, error: null };
    if (isServer) {
        return [store, () => {}];
    }

    // record each position, or the error that replaces it
    const watch = navigator.geolocation.watchPosition(
        (position) => {
            store.location = position.coords;
            store.error = null;
        },
        (error) => {
            store.location = null;
            store.error = error;
        },
        { ...DEFAULT_OPTIONS, ...options },
    );

    return [store, () => navigator.geolocation.clearWatch(watch)];
}

/** Query the position once, suspending until it answers, and again on refetch or when the options change, a seed standing in on the server. */
export function createGeolocation(
    options?: MaybeAccessor<PositionOptions>,
    initialLocation?: GeolocationCoord,
): [location: Accessor<GeolocationCoordinates>, refetch: () => void] {
    // answer the seed on the server, else stay pending
    if (isServer) {
        const seed = initialLocation === undefined ? undefined : coordinatesOf(initialLocation);

        return [
            () => {
                if (seed === undefined) {
                    throw new NotReadyError("geolocation is not available on the server");
                }

                return seed;
            },
            () => {},
        ];
    }

    // query on each version and options, suspending until the position answers
    const [version, setVersion] = createSignal(0, { ownedWrite: true });
    const location = createMemo(async () => {
        version();

        return currentPosition(access(options));
    }, TRANSPARENT);

    return [location, () => setVersion((current) => current + 1)];
}

/** Watch the position while enabled, pending until the first fix unless seeded, with the latest error, restarting as the options change. */
export function createGeolocationWatcher(
    enabled: MaybeAccessor<boolean>,
    options?: MaybeAccessor<PositionOptions>,
    initialLocation?: GeolocationCoord,
): {
    location: Accessor<GeolocationCoordinates>;
    error: Accessor<GeolocationPositionError | null>;
} {
    // hold the position, the seed, or nothing until the first fix, and the latest error
    const [current, setCurrent] = createSignal<GeolocationCoordinates | undefined>(
        initialLocation === undefined ? undefined : coordinatesOf(initialLocation),
        { ownedWrite: true },
    );
    const [error, setError] = createSignal<GeolocationPositionError | null>(null, {
        ownedWrite: true,
    });
    const location = (): GeolocationCoordinates => {
        const coordinates = current();
        if (coordinates === undefined) {
            throw new NotReadyError("waiting for the first position");
        }

        return coordinates;
    };

    // watch while enabled, with the current options, on every side to keep hydration keys aligned
    let watch: number | undefined;
    const clear = (): void => {
        if (watch !== undefined) {
            navigator.geolocation.clearWatch(watch);
            watch = undefined;
        }
    };
    createEffect(
        () => (access(enabled) ? (access(options) ?? {}) : false),
        (watched) => {
            if (isServer) {
                return;
            }
            clear();
            if (watched !== false) {
                watch = navigator.geolocation.watchPosition(
                    (position) => {
                        setCurrent(position.coords);
                        setError(null);
                    },
                    (failure) => setError(failure),
                    { ...DEFAULT_OPTIONS, ...watched },
                );
            }
        },
    );
    if (!isServer) {
        onCleanup(clear);
    }

    return { location, error };
}

/** Follow the distance from the position to a target by the haversine formula, null until the first fix. */
export function createDistance(
    target: MaybeAccessor<GeolocationCoord>,
    options: {
        readonly unit?: "km" | "m";
        readonly enabled?: MaybeAccessor<boolean>;
        readonly watcherOptions?: MaybeAccessor<PositionOptions>;
        readonly initialLocation?: GeolocationCoord;
    } = {},
): Accessor<number | null> {
    const { unit = "km", enabled = true, watcherOptions, initialLocation } = options;
    const { location } = createGeolocationWatcher(enabled, watcherOptions, initialLocation);

    return () => {
        // read no distance until the first fix
        let coordinates: GeolocationCoordinates;
        try {
            coordinates = location();
        } catch (error) {
            if (error instanceof NotReadyError) {
                return null;
            }
            throw error;
        }

        // measure in kilometres or metres
        const destination = access(target);
        const kilometres = haversine(coordinates, destination);

        return unit === "m" ? kilometres * KILOMETRE : kilometres;
    };
}

/** Follow whether the position lies within a radius in metres of a centre, false until the first fix. */
export function createWithinRadius(
    center: MaybeAccessor<GeolocationCoord>,
    radius: MaybeAccessor<number>,
    options: {
        readonly enabled?: MaybeAccessor<boolean>;
        readonly watcherOptions?: MaybeAccessor<PositionOptions>;
        readonly initialLocation?: GeolocationCoord;
    } = {},
): Accessor<boolean> {
    const distance = createDistance(center, { ...options, unit: "m" });

    return () => {
        const metres = distance();

        return metres !== null && metres <= access(radius);
    };
}

/** Query the current position, rejecting with the platform's reason. */
async function currentPosition(options?: PositionOptions): Promise<GeolocationCoordinates> {
    // refuse where the device cannot locate
    if (!("geolocation" in navigator)) {
        throw new TypeError("this browser cannot locate");
    }

    return new Promise((resolve, reject) => {
        navigator.geolocation.getCurrentPosition(
            (position) => resolve(position.coords),
            (error) => reject(new Error(`geolocation failed: ${error.message}`, { cause: error })),
            { ...DEFAULT_OPTIONS, ...options },
        );
    });
}

/** Stand a seed position in for measured coordinates, whose accuracy is unmeasured and so spans any distance. */
function coordinatesOf(seed: GeolocationCoord): GeolocationCoordinates {
    const coordinates = {
        latitude: seed.latitude,
        longitude: seed.longitude,
        altitude: null,
        accuracy: Number.POSITIVE_INFINITY,
        altitudeAccuracy: null,
        heading: null,
        speed: null,
    };

    return { ...coordinates, toJSON: () => coordinates };
}

/** Turn degrees into radians. */
function radians(degrees: number): number {
    return (degrees * Math.PI) / 180;
}

/** Measure the great-circle distance between two positions in kilometres. */
function haversine(from: GeolocationCoord, to: GeolocationCoord): number {
    // take the differences of latitude and longitude in radians
    const latitude = radians(to.latitude - from.latitude);
    const longitude = radians(to.longitude - from.longitude);
    const chord =
        Math.sin(latitude / 2) ** 2 +
        Math.cos(radians(from.latitude)) *
            Math.cos(radians(to.latitude)) *
            Math.sin(longitude / 2) ** 2;

    return EARTH_RADIUS * 2 * Math.atan2(Math.sqrt(chord), Math.sqrt(1 - chord));
}
