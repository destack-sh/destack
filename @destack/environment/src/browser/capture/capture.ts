/** A browser's cameras, microphones and screens, as `navigator.mediaDevices` has them. */
export interface Capture {
    /** List the media input and output devices. */
    readonly enumerateDevices: () => Promise<MediaDeviceInfo[]>;
    /** Capture a camera or microphone stream. */
    readonly getUserMedia: (constraints?: MediaStreamConstraints) => Promise<MediaStream>;
    /** Capture a screen, window or tab stream. */
    readonly getDisplayMedia: (options?: DisplayMediaStreamOptions) => Promise<MediaStream>;
    /** Follow each change of the device list, until disposed. */
    readonly on: (listener: () => void) => Disposable;
}

/** Captures. */
export const Capture: { readonly system: Capture } = {
    /** The browser's media devices. */
    system: {
        enumerateDevices: () => navigator.mediaDevices.enumerateDevices(),
        getUserMedia: (constraints) => navigator.mediaDevices.getUserMedia(constraints),
        getDisplayMedia: (options) => navigator.mediaDevices.getDisplayMedia(options),
        on: (listener) => {
            navigator.mediaDevices.addEventListener("devicechange", listener);

            return {
                [Symbol.dispose]: () =>
                    navigator.mediaDevices.removeEventListener("devicechange", listener),
            };
        },
    },
};
