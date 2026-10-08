import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal, onCleanup } from "solid-js";
import { access, type MaybeAccessor } from "./utils.ts";

/** A vibration in milliseconds, or alternating vibration and pause durations. */
export type VibratePattern = number | number[];

/** How a vibration repeats. */
export interface VibrateOptions {
    /** The milliseconds between repetitions, once per start without it. */
    readonly interval?: number;
}

/** How a pulse vibrates. */
export interface PulseOptions {
    /** The share of each cycle spent vibrating, from zero to one, half by default. */
    readonly dutyCycle?: number;
}

/** The length of one repeating stretch of a pulse, in milliseconds. */
const PULSE_CHUNK = 1000;

/** The milliseconds in a second, which a frequency divides into a period. */
const SECOND = 1000;

/** The share of each pulse cycle that vibrates, half by default. */
const DUTY_CYCLE = 0.5;

/** The slowest pulse, once every ten seconds, below which a pulse reads as separate vibrations. */
const MIN_PULSE_HZ = 0.1;

/** The fastest pulse, one cycle every ten milliseconds, about the shortest vibration devices render. */
const MAX_PULSE_HZ = 100;

/** Report whether the device can vibrate. */
export function isVibrationSupported(): boolean {
    return !isServer && "vibrate" in navigator;
}

/** Make functions that start and stop a vibration, repeating it every interval when given, doing nothing where the device cannot vibrate. */
export function makeVibrate(
    pattern: VibratePattern,
    options: VibrateOptions = {},
): [start: () => void, stop: () => void] {
    // vibrate nothing where the device cannot
    if (!isVibrationSupported()) {
        return [() => {}, () => {}];
    }

    // vibrate, repeating every interval, and stop by vibrating for zero
    let handle: ReturnType<typeof setInterval> | undefined;
    const stop = (): void => {
        clearInterval(handle);
        handle = undefined;
        navigator.vibrate(0);
    };
    const start = (): void => {
        navigator.vibrate(pattern);
        if (options.interval !== undefined) {
            clearInterval(handle);
            handle = setInterval(() => navigator.vibrate(pattern), options.interval);
        }
    };

    return [start, stop];
}

/** Vibrate in a pattern, restarting with a reactive pattern as it changes and stopping on cleanup, telling whether the device can vibrate. */
export function createVibrate(
    pattern: MaybeAccessor<VibratePattern>,
    options: VibrateOptions = {},
): { vibrating: Accessor<boolean>; start: () => void; stop: () => void; supported: boolean } {
    // follow the pattern on every side, keeping hydration keys aligned, and vibrate only where the device can
    const supported = isVibrationSupported();
    const [vibrating, setVibrating] = createSignal(false, { ownedWrite: true });
    let isVibrating = false;
    let handle: ReturnType<typeof setInterval> | undefined;
    const vibrate = (current: VibratePattern): void => {
        navigator.vibrate(current);
        if (options.interval !== undefined) {
            clearInterval(handle);
            handle = setInterval(() => navigator.vibrate(current), options.interval);
        }
    };

    // restart with a changed pattern while vibrating, skipping the first value
    if (typeof pattern === "function") {
        let isInitial = true;
        createEffect(pattern, (current) => {
            if (isInitial) {
                isInitial = false;
            } else if (supported && isVibrating) {
                vibrate(current);
            }
        });
    }
    if (!supported) {
        return { vibrating: () => false, start: () => {}, stop: () => {}, supported };
    }

    // start and stop, stopping on cleanup
    const stop = (): void => {
        // stop repeating and stop the device
        clearInterval(handle);
        handle = undefined;
        navigator.vibrate(0);
        isVibrating = false;
        setVibrating(false);
    };
    const start = (): void => {
        vibrate(access(pattern));
        isVibrating = true;
        setVibrating(true);
    };
    onCleanup(stop);

    return { vibrating, start, stop, supported };
}

/** Turn a frequency and a duty cycle into one cycle of vibration and pause, in milliseconds. */
export function frequencyToPattern(hz: number, dutyCycle = DUTY_CYCLE): [on: number, off: number] {
    const period = SECOND / Math.max(0.001, hz);
    const share = Math.min(Math.max(dutyCycle, 0), 1);

    return [Math.max(1, Math.round(period * share)), Math.max(1, Math.round(period * (1 - share)))];
}

/** Make functions that start and stop a vibration pulsing a number of times a second. */
export function makePulse(
    hz: number,
    options: PulseOptions = {},
): [start: () => void, stop: () => void] {
    // pulse nothing where the device cannot vibrate
    if (!isVibrationSupported()) {
        return [() => {}, () => {}];
    }
    const { pattern, duration } = pulseChunk(hz, options.dutyCycle ?? DUTY_CYCLE);

    return makeVibrate(pattern, { interval: duration });
}

/** Pulse a number of times a second, restarting with a reactive frequency as it changes and stopping on cleanup, telling whether the device can vibrate. */
export function createPulse(
    hz: MaybeAccessor<number>,
    options: PulseOptions = {},
): { pulsing: Accessor<boolean>; start: () => void; stop: () => void; supported: boolean } {
    // follow the frequency on every side, keeping hydration keys aligned, and pulse only where the device can
    const supported = isVibrationSupported();
    const dutyCycle = options.dutyCycle ?? DUTY_CYCLE;
    const [pulsing, setPulsing] = createSignal(false, { ownedWrite: true });
    let isPulsing = false;
    let handle: ReturnType<typeof setInterval> | undefined;
    const pulse = (current: number): void => {
        // vibrate a stretch of cycles, repeating it
        const { pattern, duration } = pulseChunk(current, dutyCycle);
        navigator.vibrate(pattern);
        clearInterval(handle);
        handle = setInterval(() => navigator.vibrate(pattern), duration);
    };

    // restart at a changed frequency while pulsing, skipping the first value
    if (typeof hz === "function") {
        let isInitial = true;
        createEffect(hz, (current) => {
            if (isInitial) {
                isInitial = false;
            } else if (supported && isPulsing) {
                pulse(current);
            }
        });
    }
    if (!supported) {
        return { pulsing: () => false, start: () => {}, stop: () => {}, supported };
    }

    // start and stop, stopping on cleanup
    const stop = (): void => {
        // stop repeating and stop the device
        clearInterval(handle);
        handle = undefined;
        navigator.vibrate(0);
        isPulsing = false;
        setPulsing(false);
    };
    const start = (): void => {
        pulse(access(hz));
        isPulsing = true;
        setPulsing(true);
    };
    onCleanup(stop);

    return { pulsing, start, stop, supported };
}

/** Build a stretch of pulse cycles about a second long, with its exact length. */
function pulseChunk(hz: number, dutyCycle: number): { pattern: number[]; duration: number } {
    const [on, off] = frequencyToPattern(
        Math.min(Math.max(hz, MIN_PULSE_HZ), MAX_PULSE_HZ),
        dutyCycle,
    );
    const cycles = Math.max(1, Math.round(PULSE_CHUNK / (on + off)));

    return {
        pattern: Array.from({ length: cycles }, () => [on, off]).flat(),
        duration: cycles * (on + off),
    };
}
