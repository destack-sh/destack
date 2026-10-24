/** A browser's position, as `navigator.geolocation` has it. */
export interface Geolocation {
    /** Read the current position. */
    readonly getCurrentPosition: (options?: PositionOptions) => Promise<GeolocationPosition>;
    /** Follow the position and its errors, until disposed. */
    readonly watchPosition: (
        listener: (position: GeolocationPosition) => void,
        error: (error: GeolocationPositionError) => void,
        options?: PositionOptions,
    ) => Disposable;
}

/** Geolocations. */
export const Geolocation: { readonly system: Geolocation } = {
    /** The browser's geolocation. */
    system: {
        getCurrentPosition: (options) =>
            new Promise((resolve, reject) => {
                navigator.geolocation.getCurrentPosition(resolve, reject, options);
            }),
        watchPosition: (listener, error, options) => {
            const watch = navigator.geolocation.watchPosition(listener, error, options);

            return { [Symbol.dispose]: () => navigator.geolocation.clearWatch(watch) };
        },
    },
};
