import type { Accessor } from "solid-js";
import { createSignal, onCleanup, onMount } from "solid-js";

/// Configuration for one deterministic column sweep.
export type SweepOptions = {
    /// The first sweep column.
    firstColumn: number;
    /// The final sweep column.
    lastColumn: number;
    /// The delay between column changes.
    stepMilliseconds: number;
};

/// Create one reduced-motion-aware column sweep.
export function createSweep(options: SweepOptions): Accessor<number> {
    const [column, setColumn] = createSignal(options.firstColumn);

    // advance the shared phase after hydration
    onMount(() => {
        if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
            return;
        }

        const interval = window.setInterval(() => {
            setColumn((currentColumn) => {
                const nextColumn = currentColumn + 1;

                return nextColumn > options.lastColumn ? options.firstColumn : nextColumn;
            });
        }, options.stepMilliseconds);

        onCleanup(() => window.clearInterval(interval));
    });

    return column;
}
