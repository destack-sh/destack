import { type Accessor, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { contentsStyles } from "./publication.stylex";

/// One article heading in the rendered contents.
export type ContentsEntry = {
    /// The heading depth.
    depth: number;

    /// The heading identifier.
    id: string;

    /// The visible heading text.
    text: string;
};

/// Properties for one article contents list.
type ContentsProps = {
    /// The currently active heading identifier.
    activeId: Accessor<string>;

    /// The document headings.
    entries: readonly ContentsEntry[];

    /// Whether the contents render inside a compact menu.
    isMenu?: boolean;
};

/// Render the active article outline.
export function Contents(props: ContentsProps) {
    return (
        <Show when={props.entries.length > 0}>
            <nav
                aria-label="contents"
                {...stylex.attrs(contentsStyles.root, props.isMenu && contentsStyles.rootMenu)}
            >
                <p {...stylex.attrs(contentsStyles.heading)}>contents</p>
                <ol {...stylex.attrs(contentsStyles.list)}>
                    <For each={props.entries}>
                        {(entry) => (
                            <li {...stylex.attrs(entry.depth > 2 && contentsStyles.nested)}>
                                <a
                                    {...stylex.attrs(
                                        contentsStyles.link,
                                        props.activeId() === entry.id && contentsStyles.active,
                                    )}
                                    href={`#${entry.id}`}
                                >
                                    {entry.text}
                                </a>
                            </li>
                        )}
                    </For>
                </ol>
            </nav>
        </Show>
    );
}

/// Track the last heading above the reading position.
export function trackActiveHeading(entries: readonly ContentsEntry[]) {
    const [activeId, setActiveId] = createSignal(entries[0]?.id ?? "");

    onMount(() => {
        // skip documents without headings
        if (entries.length === 0) {
            return;
        }

        let frame = 0;

        // update at most once per rendered frame
        const update = () => {
            frame = 0;
            setActiveId(visibleHeading(entries));
        };

        // coalesce repeated viewport events
        const schedule = () => {
            if (frame === 0) {
                frame = window.requestAnimationFrame(update);
            }
        };

        // initialize and bind viewport tracking
        update();
        window.addEventListener("scroll", schedule, { passive: true });
        window.addEventListener("resize", schedule);

        // cancel pending work and detach viewport tracking
        onCleanup(() => {
            if (frame !== 0) {
                window.cancelAnimationFrame(frame);
            }

            window.removeEventListener("scroll", schedule);
            window.removeEventListener("resize", schedule);
        });
    });

    return activeId;
}

/// Find the last heading above the top navigation.
function visibleHeading(entries: readonly ContentsEntry[]) {
    // begin at the first authored heading
    const offset = 96;
    let current = entries[0]?.id ?? "";

    // advance through headings above the reading position
    for (const entry of entries) {
        const element = document.getElementById(entry.id);

        // ignore headings absent from the rendered document
        if (element == undefined) {
            continue;
        }

        // stop at the first heading below the reading position
        if (element.getBoundingClientRect().top > offset) {
            break;
        }

        current = entry.id;
    }

    return current;
}
