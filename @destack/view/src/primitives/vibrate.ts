import type { Clock } from "@destack/environment";
import { type Accessor, createEffect, createSignal, onCleanup, untrack } from "solid-js";
import { useEnvironment } from "../page/view.ts";
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

/** A vibration and how often it repeats. */
interface Vibration {
    /** The pattern. */
    readonly pattern: VibratePattern;
    /** The milliseconds between repetitions, once without it. */
    readonly interval?: number;
}

/** The length of one repeating stretch of a pulse, in milliseconds. */
const PULSE_CHUNK = 1000;

/** The milliseconds in a second, which a frequency divides into a period. */
const SECOND = 1000;

/** The share of each pulse cycle that vibrates, half by default. */
const DUTY_CYCLE = 0.5;

/** The slowest pulse, once every ten seconds, below which a pulse reads as separate vibrations. */
const MIN_PULSE_HZ = 0.1;

/** The fastest pulse, one cycle every ten milliseconds. */
const MAX_PULSE_HZ = 100;

/** The device's vibration motor repeating a vibration on a clock. */
class Vibrator {
    /** The clock repeats run on. */
    readonly #clock: Clock;
    /** The handle of the repeating vibration, absent while none repeats. */
    #handle: number | undefined;

    /** Drive the device's motor on a clock. */
    constructor(clock: Clock) {
        this.#clock = clock;
        this.#handle = undefined;
    }

    /** Vibrate, repeating every interval when given, reporting whether the device vibrates. */
    start(vibration: Vibration): boolean {
        // vibrate once, and repeat where the device vibrated and an interval is given
        this.#stopRepeating();
        const isStarted = vibrate(vibration.pattern);
        if (isStarted && vibration.interval !== undefined) {
            this.#handle = this.#clock.setInterval(
                () => vibrate(vibration.pattern),
                vibration.interval,
            );
        }

        return isStarted;
    }

    /** Stop repeating and stop the device. */
    stop(): void {
        this.#stopRepeating();
        vibrate(0);
    }

    /** Stop the repeating vibration. */
    #stopRepeating(): void {
        if (this.#handle !== undefined) {
            this.#clock.clearInterval(this.#handle);
            this.#handle = undefined;
        }
    }
}

/** Make functions that start and stop a vibration, repeating it on a clock at an interval. */
export function makeVibrate(
    pattern: VibratePattern,
    options: VibrateOptions,
    clock: Clock,
): [start: () => void, stop: () => void] {
    const vibrator = new Vibrator(clock);

    return [() => vibrator.start({ pattern, ...options }), () => vibrator.stop()];
}

/** Vibrate in a pattern, restarting with a reactive pattern as it changes. */
export function createVibrate(
    pattern: MaybeAccessor<VibratePattern>,
    options: VibrateOptions = {},
): { vibrating: Accessor<boolean>; start: () => void; stop: () => void } {
    const { active, start, stop } = createVibration(() => ({
        pattern: access(pattern),
        ...options,
    }));

    return { vibrating: active, start, stop };
}

/** Turn a frequency and a duty cycle into one cycle of vibration and pause, in milliseconds. */
export function frequencyToPattern(hz: number, dutyCycle = DUTY_CYCLE): [on: number, off: number] {
    const period = SECOND / Math.max(0.001, hz);
    const share = Math.min(Math.max(dutyCycle, 0), 1);

    return [Math.max(1, Math.round(period * share)), Math.max(1, Math.round(period * (1 - share)))];
}

/** Make functions that start and stop a vibration pulsing a number of times a second on a clock. */
export function makePulse(
    hz: number,
    options: PulseOptions,
    clock: Clock,
): [start: () => void, stop: () => void] {
    const vibrator = new Vibrator(clock);
    const pulse = pulseOf(hz, options.dutyCycle ?? DUTY_CYCLE);

    return [() => vibrator.start(pulse), () => vibrator.stop()];
}

/** Pulse a number of times a second, restarting with a reactive frequency as it changes. */
export function createPulse(
    hz: MaybeAccessor<number>,
    options: PulseOptions = {},
): { pulsing: Accessor<boolean>; start: () => void; stop: () => void } {
    const dutyCycle = options.dutyCycle ?? DUTY_CYCLE;
    const { active, start, stop } = createVibration(() => pulseOf(access(hz), dutyCycle));

    return { pulsing: active, start, stop };
}

/** Vibrate as a reactive vibration reads, restarting as it changes, stopping on cleanup. */
function createVibration(read: () => Vibration): {
    active: Accessor<boolean>;
    start: () => void;
    stop: () => void;
} {
    // vibrate on the page's motor, which the server never starts
    const vibrator = new Vibrator(useEnvironment().clock);
    const [active, setActive] = createSignal(false, { ownedWrite: true });

    // restart with a changed vibration while active, skipping the first value
    let isInitial = true;
    createEffect(read, (current) => {
        if (isInitial) {
            isInitial = false;
        } else if (untrack(active)) {
            vibrator.start(current);
        }
    });

    // start and stop, stopping on cleanup
    const stop = (): void => {
        vibrator.stop();
        setActive(false);
    };
    const start = (): void => {
        setActive(vibrator.start(untrack(read)));
    };
    onCleanup(() => {
        if (untrack(active)) {
            stop();
        }
    });

    return { active, start, stop };
}

/** Build a stretch of pulse cycles about a second long, repeating every stretch. */
function pulseOf(hz: number, dutyCycle: number): Vibration {
    const [on, off] = frequencyToPattern(
        Math.min(Math.max(hz, MIN_PULSE_HZ), MAX_PULSE_HZ),
        dutyCycle,
    );
    const cycles = Math.max(1, Math.round(PULSE_CHUNK / (on + off)));

    return {
        pattern: Array.from({ length: cycles }, () => [on, off]).flat(),
        interval: cycles * (on + off),
    };
}

/** Vibrate the device in a pattern, reporting whether it started, false without a motor. */
function vibrate(pattern: VibratePattern): boolean {
    return typeof navigator.vibrate === "function" && navigator.vibrate(pattern);
}
