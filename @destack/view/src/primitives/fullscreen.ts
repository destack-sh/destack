import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal, onCleanup } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { access, TRANSPARENT } from "./utils.ts";

/** How fullscreen opens, and whether it closes with the owner. */
export interface FullscreenPrimitiveOptions extends FullscreenOptions {
    /** Whether fullscreen closes when the owner is cleaned up, true by default. */
    readonly exitOnCleanup?: boolean;
}

/** Make functions that enter and exit fullscreen for an element, the entering options overriding the made ones. */
export function makeFullscreen(
    element: HTMLElement,
    options?: FullscreenOptions,
): [enter: (options?: FullscreenOptions) => Promise<void>, exit: () => Promise<void>] {
    return [
        async (overrides) => element.requestFullscreen(overrides ?? options),
        async () => document.exitFullscreen(),
    ];
}

/** Enter and exit fullscreen for an element, following whether it is fullscreen. */
export function createFullscreen(
    target: HTMLElement | Accessor<HTMLElement | undefined>,
    options?: FullscreenPrimitiveOptions,
): {
    enter: (options?: FullscreenOptions) => Promise<void>;
    exit: () => Promise<void>;
    isActive: Accessor<boolean>;
} {
    // open nothing on the server
    if (isServer) {
        return { enter: async () => {}, exit: async () => {}, isActive: () => false };
    }

    // follow whether the element is the fullscreen one, from now on
    const { exitOnCleanup = true, ...native } = options ?? {};
    const initial = access(target);
    let bound = initial === undefined ? undefined : makeFullscreen(initial, native);
    const [isActive, setIsActive] = createSignal(
        initial !== undefined && document.fullscreenElement === initial,
        { ownedWrite: true },
    );
    makeEventListener(document, "fullscreenchange", () =>
        setIsActive(document.fullscreenElement === access(target)),
    );

    // bind to each element an accessor gives
    if (typeof target === "function") {
        createEffect(
            target,
            (element) => {
                bound = element === undefined ? undefined : makeFullscreen(element, native);
                setIsActive(element !== undefined && document.fullscreenElement === element);
            },
            TRANSPARENT,
        );
    }

    // close fullscreen with the owner unless told otherwise, reporting a refusal
    onCleanup(() => {
        if (
            exitOnCleanup &&
            document.fullscreenElement !== null &&
            document.fullscreenElement === access(target)
        ) {
            document.exitFullscreen().catch(reportError);
        }
    });

    return {
        enter: async (overrides) => {
            if (bound === undefined) {
                throw new TypeError("there is no element to show in fullscreen");
            }
            await bound[0](overrides);
        },
        exit: async () => {
            if (bound === undefined) {
                throw new TypeError("there is no element to take out of fullscreen");
            }
            await bound[1]();
        },
        isActive,
    };
}

/** Toggle fullscreen for the element the ref receives on each click. */
export function fullscreen(options?: FullscreenOptions): (element: HTMLElement) => void {
    // toggle nothing on the server
    if (isServer) {
        return () => {};
    }

    // toggle on click of the latest element, reporting a refusal, until cleanup
    let stop: (() => void) | undefined;
    onCleanup(() => stop?.());

    return (element) => {
        // toggle on the new element only
        stop?.();
        const [enter, exit] = makeFullscreen(element, options);
        const toggle = (): void => {
            (document.fullscreenElement === element ? exit() : enter()).catch(reportError);
        };
        element.addEventListener("click", toggle);
        stop = () => element.removeEventListener("click", toggle);
    };
}
