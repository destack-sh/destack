import { isServer } from "@solidjs/web";
import { type Accessor, createSignal, onCleanup } from "solid-js";
import { onMount } from "./utils.ts";

/** An event that shows the person is active. */
export type EventTypeName = keyof HTMLElementEventMap | keyof DocumentEventMap;

/** How an idle timer watches activity and when it prompts and goes idle. */
export interface IdleTimerOptions {
    /** The events that count as activity, pointer, key, wheel, touch, resize and visibility by default. */
    readonly events?: EventTypeName[];
    /** The element to watch, the document by default. */
    readonly element?: HTMLElement;
    /** How long without activity before the prompt, fifteen minutes by default. */
    readonly idleTimeout?: number;
    /** How long the prompt lasts before going idle, none by default. */
    readonly promptTimeout?: number;
    /** Whether the timer waits for `start` instead of starting once mounted. */
    readonly startManually?: boolean;
    /** Handle going idle, with the last activity before the prompt. */
    readonly onIdle?: (lastEvent: Event) => void;
    /** Handle activity after going idle. */
    readonly onActive?: (activityEvent: Event) => void;
    /** Handle the prompt, with the last activity before it. */
    readonly onPrompt?: (promptEvent: Event) => void;
}

/** An idle timer's phase and controls. */
export interface IdleTimer {
    /** Whether the person is idle. */
    readonly isIdle: Accessor<boolean>;
    /** Whether the prompt runs. */
    readonly isPrompted: Accessor<boolean>;
    /** Restart the timers without reporting activity. */
    readonly reset: () => void;
    /** Listen and start the timers. */
    readonly start: () => void;
    /** Stop listening and clear the timers. */
    readonly stop: () => void;
    /** Go idle now, reporting it without a prompt. */
    readonly triggerIdle: () => void;
}

/** How often activity events are read, at most, in milliseconds. */
const THROTTLE_DELAY = 250;

/** The default time without activity before the prompt: fifteen minutes. */
const FIFTEEN_MINUTES = 15 * 60 * 1000;

/** The events that count as activity by default. */
const EVENTS: EventTypeName[] = [
    "mousemove",
    "keydown",
    "wheel",
    "resize",
    "mousedown",
    "pointerdown",
    "touchstart",
    "touchmove",
    "visibilitychange",
];

/** Follow whether the person is idle, prompting before going idle. */
export function createIdleTimer({
    element,
    events = EVENTS,
    idleTimeout = FIFTEEN_MINUTES,
    promptTimeout = 0,
    onActive,
    onIdle,
    onPrompt,
    startManually = false,
}: IdleTimerOptions = {}): IdleTimer {
    // run nothing on the server
    if (isServer) {
        return {
            isIdle: () => false,
            isPrompted: () => false,
            reset: () => {},
            start: () => {},
            stop: () => {},
            triggerIdle: () => {},
        };
    }

    // keep the phase in plain state, which the timers read right away, mirrored to signals
    const [isIdle, setIsIdle] = createSignal(false, { ownedWrite: true });
    const [isPrompted, setIsPrompted] = createSignal(false, { ownedWrite: true });
    let isIdleNow = false;
    let isPromptedNow = false;
    const phase = (nextIdle: boolean, nextPrompted: boolean): void => {
        // write both the plain state and its signals
        isIdleNow = nextIdle;
        isPromptedNow = nextPrompted;
        setIsIdle(nextIdle);
        setIsPrompted(nextPrompted);
    };

    // prompt after the idle timeout, then go idle after the prompt timeout
    let idleHandle: ReturnType<typeof setTimeout> | undefined;
    let promptHandle: ReturnType<typeof setTimeout> | undefined;
    const clearTimers = (): void => {
        clearTimeout(idleHandle);
        clearTimeout(promptHandle);
    };
    const startTimers = (event: Event): void => {
        idleHandle = setTimeout(() => {
            phase(false, true);
            onPrompt?.(event);
            promptHandle = setTimeout(() => {
                phase(true, false);
                onIdle?.(event);
            }, promptTimeout);
        }, idleTimeout);
    };

    // restart the timers on activity, reporting activity after idleness, and leaving a running prompt alone
    let isListening = false;
    let lastActivity = Number.NEGATIVE_INFINITY;
    const restart = (event: Event): void => {
        if (isListening) {
            clearTimers();
            phase(false, false);
            startTimers(event);
        }
    };
    const handle = (event: Event): void => {
        // read activity at most once a throttle delay
        const now = performance.now();
        if (now - lastActivity < THROTTLE_DELAY) {
            return;
        }
        lastActivity = now;
        if (isIdleNow) {
            onActive?.(event);
        }
        if (!isPromptedNow) {
            restart(event);
        }
    };

    // listen to activity on the element or the document
    const target = element ?? document;
    const listen = (): void => {
        if (!isListening) {
            for (const name of events) {
                target.addEventListener(name, handle);
            }
            isListening = true;
        }
    };
    const unlisten = (): void => {
        if (isListening) {
            for (const name of events) {
                target.removeEventListener(name, handle);
            }
            isListening = false;
        }
    };

    // start, stop and go idle by hand
    const start = (event: Event = new CustomEvent("manualstart")): void => {
        // start the timers from scratch
        clearTimers();
        phase(false, false);
        listen();
        restart(event);
    };
    const stop = (): void => {
        clearTimers();
        phase(false, false);
        unlisten();
    };
    const triggerIdle = (): void => {
        // stop, go idle, and listen for the activity that ends it
        stop();
        phase(true, false);
        onIdle?.(new CustomEvent("manualidle"));
        listen();
    };

    // start once mounted unless told otherwise, and stop on cleanup
    onMount(() => {
        if (!startManually) {
            start(new CustomEvent("mount"));
        }
    });
    onCleanup(stop);

    return {
        isIdle,
        isPrompted,
        start: () => start(),
        reset: () => restart(new CustomEvent("manualreset")),
        stop,
        triggerIdle,
    };
}
