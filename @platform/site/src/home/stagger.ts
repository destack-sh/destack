import { createEffect, createSignal } from "@destack/view";

/** Whether the stack is open at a numbered step of a figure's switch. */
export type Stagger = (step: number) => boolean;

/**
 * Follow the stack's state step by step, so a figure switches its parts in the order of their numbers.
 *
 * Step zero switches at once and every later step one interval after the one before it, up to the last.
 * With reduced motion, every step switches at once.
 */
export function createStagger(isOpen: () => boolean, last: number, interval: number): Stagger {
    // hold the state the walk switches to and the last step to reach it
    let current = isOpen();
    const [target, setTarget] = createSignal(current);
    const [reached, setReached] = createSignal(Number.POSITIVE_INFINITY);

    // walk the steps to every new state one interval apart
    createEffect(isOpen, (open) => {
        // keep a state already reached
        if (open === current) {
            return undefined;
        }
        current = open;
        setTarget(open);
        if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
            setReached(Number.POSITIVE_INFINITY);
            return undefined;
        }

        // reach the next step on every interval until the last
        let step = 0;
        setReached(step);
        const timer = setInterval(() => {
            step += 1;
            setReached(step > last ? Number.POSITIVE_INFINITY : step);
            if (step > last) {
                clearInterval(timer);
            }
        }, interval);

        return () => clearInterval(timer);
    });

    return (step) => (step <= reached() ? target() : !target());
}
