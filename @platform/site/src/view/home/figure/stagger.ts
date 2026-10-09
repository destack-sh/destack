import { createEffect, createSignal } from "@destack/view";
import { makeTimer } from "@destack/view/primitives/timer";

/** Whether the stack is open at a numbered step of a figure's switch. */
export type Stagger = (step: number) => boolean;

/** A figure's stack state by step: whether each step is open, and how to switch one step alone. */
type Steps = { isOpenAt: Stagger; toggle: (step: number) => void };

/**
 * Follow the stack's state step by step, so a figure switches its parts in the order of their numbers.
 *
 * Step zero switches at once and every later step one interval after the one before it, up to the last.
 * Any step also switches alone on request, until the stack switches again.
 * With reduced motion, every step switches at once.
 */
export function createSteps(
    isOpen: () => boolean,
    last: number,
    interval: number,
    isReduced: () => boolean,
): Steps {
    // hold whether each step is open, mirrored for the walk to read
    let current: readonly boolean[] = Array.from({ length: last + 1 }, () => isOpen());
    const [states, setStates] = createSignal(current);
    const set = (next: readonly boolean[]) => {
        current = next;
        setStates(next);
    };

    // walk the steps to every new state one interval apart
    createEffect(
        () => ({ open: isOpen(), isStill: isReduced() }),
        ({ open, isStill }) => {
            // keep the steps when every one already holds the state, and switch them all at once while motion is reduced
            if (current.every((state) => state === open)) {
                return undefined;
            }
            if (isStill) {
                set(current.map(() => open));
                return undefined;
            }

            // reach the next step on every interval until the last
            let step = 0;
            const reach = () => set(current.map((state, index) => (index === step ? open : state)));
            reach();
            const clear = makeTimer(
                () => {
                    step += 1;
                    reach();
                    if (step >= last) {
                        clear();
                    }
                },
                interval,
                setInterval,
            );

            return clear;
        },
    );

    return {
        isOpenAt: (step) => states()[step] ?? false,
        toggle: (step) => set(current.map((state, index) => (index === step ? !state : state))),
    };
}
