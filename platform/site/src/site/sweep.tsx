import type { Accessor } from "solid-js";
import { createSignal, onCleanup, onMount } from "solid-js";

/// The distance between the primary sweep and its trailing coordinate.
const trailDistance = 2;

/// Configuration for one deterministic column sweep.
export type SweepOptions = {
    /// The first sweep column.
    firstColumn: number;
    /// The final sweep column.
    lastColumn: number;
    /// The delay between column changes.
    stepMilliseconds: number;
};

/// One automatic sweep that can be steered temporarily.
export type Sweep = {
    /// The current sweep column.
    column: Accessor<number>;

    /// The monotonically advancing sweep phase.
    phase: Accessor<number>;

    /// Resume automatic movement from the current column.
    release: () => void;

    /// Move the sweep directly to one column.
    steer: (column: number) => void;

    /// The coordinate trailing behind the current sweep direction.
    trailColumn: Accessor<number>;
};

/// Create one reduced-motion-aware, steerable column sweep.
export function createSweep(options: SweepOptions): Sweep {
    const [column, setColumn] = createSignal(options.firstColumn);
    const [phase, setPhase] = createSignal(0);
    const [trailColumn, setTrailColumn] = createSignal(options.firstColumn);
    let isSteered = false;

    // advance the shared phase while no pointer is steering it
    onMount(() => {
        if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
            return;
        }

        const interval = window.setInterval(() => {
            setPhase((currentPhase) => currentPhase + 1);

            if (isSteered) {
                return;
            }

            const nextColumn = wrapColumn(column() + 1, options);
            const nextTrailColumn = wrapColumn(nextColumn - trailDistance, options);
            setColumn(nextColumn);
            setTrailColumn(nextTrailColumn);
        }, options.stepMilliseconds);

        onCleanup(() => window.clearInterval(interval));
    });

    // move the primary light immediately and retain a short directional trail
    const steer = (nextColumn: number) => {
        const clampedColumn = Math.max(
            options.firstColumn,
            Math.min(options.lastColumn, nextColumn),
        );
        const direction = Math.sign(clampedColumn - column());
        const nextTrailColumn = Math.max(
            options.firstColumn,
            Math.min(options.lastColumn, clampedColumn - direction * trailDistance),
        );
        isSteered = true;
        setColumn(clampedColumn);
        setTrailColumn(nextTrailColumn);
    };

    // continue the idle sweep from its current position
    const release = () => {
        isSteered = false;
    };

    return { column, phase, release, steer, trailColumn };
}

/// Wrap one column around the configured sweep interval.
function wrapColumn(column: number, options: SweepOptions) {
    const sweepWidth = options.lastColumn - options.firstColumn + 1;
    const offset = column - options.firstColumn;
    const wrappedOffset = ((offset % sweepWidth) + sweepWidth) % sweepWidth;

    return wrappedOffset + options.firstColumn;
}
