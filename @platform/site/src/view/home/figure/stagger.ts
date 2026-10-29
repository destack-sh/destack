import { createEffect, createSignal, untrack, useEnvironment } from "@destack/view";
import { makeTimer } from "@destack/view/primitives/timer";

/** The milliseconds a figure's switch takes to reach its last step, the medium motion duration. */
const WALK_TIME = 300;

/** Whether the stack is open at a numbered step of a figure's switch. */
export type Stagger = (step: number) => boolean;

/** A figure's stack state by step: whether each step is open, and how to switch one step alone. */
type Steps = { isOpenAt: Stagger; toggle: (step: number) => void };

/** Follow the stack's state step by step. */
export function createSteps(isOpen: () => boolean, last: number, isReduced: () => boolean): Steps {
    // hold whether each step is open, seeded once from the state at creation and mirrored for the walk to read
    let current: readonly boolean[] = Array.from({ length: last + 1 }, () => untrack(isOpen));
    const { clock } = useEnvironment();
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

            // reach the next step on every interval until the last, so every walk ends together
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
                WALK_TIME / last,
                "interval",
                clock,
            );

            return clear;
        },
    );

    return {
        isOpenAt: (step) => states()[step] ?? false,
        toggle: (step) => set(current.map((state, index) => (index === step ? !state : state))),
    };
}
