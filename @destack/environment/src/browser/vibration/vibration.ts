/** A device's vibration motor, as the Vibration API has it. */
export interface Vibration {
    /** Vibrate in a pattern of milliseconds, reporting whether it started; 0 stops it. */
    readonly vibrate: (pattern: VibratePattern) => boolean;
}

/** Vibrations. */
export const Vibration: { readonly system: Vibration } = {
    /** The browser's vibration motor. */
    system: {
        vibrate: (pattern) => navigator.vibrate(pattern),
    },
};
