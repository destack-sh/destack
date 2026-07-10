import type { Accessor } from "solid-js";
import { createSignal, onCleanup, onMount, Show } from "solid-js";

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

/// Properties for text illuminated by a column sweep.
export type SweepTextProps = {
    /// The current shared sweep column.
    column: number;
    /// The optional element class.
    class?: string;
    /// The first column occupied by the text.
    firstColumn: number;
    /// The displayed text.
    text: string;
};

/// Render text with one illuminated character at a time.
export function SweepText(props: SweepTextProps) {
    const [isMounted, setIsMounted] = createSignal(false);

    // preserve one stable text node through server hydration
    onMount(() => setIsMounted(true));

    return (
        <span class={props.class}>
            <Show fallback={props.text} when={isMounted()}>
                {Array.from(props.text, (character, index) => {
                    const column = props.firstColumn + index;
                    const isLit = Math.abs(props.column - column) <= 1;

                    return (
                        <span classList={{ "sweep-text__character--lit": isLit }}>
                            {character}
                        </span>
                    );
                })}
            </Show>
        </span>
    );
}
