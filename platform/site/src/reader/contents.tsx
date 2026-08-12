import {
    type Accessor,
    createMemo,
    createSignal,
    For,
    onCleanup,
    onMount,
    Show,
} from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

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
    const entries = createMemo(() =>
        props.isMenu ? props.entries : visibleContents(props.entries, props.activeId())
    );

    return (
        <Show when={props.entries.length > 0}>
            <nav
                aria-label="contents"
                {...stylex.attrs(styles.root, props.isMenu && styles.rootMenu)}
            >
                <p {...stylex.attrs(styles.heading)}>contents</p>
                <ol {...stylex.attrs(styles.list)}>
                    <For each={entries()}>
                        {(entry) => (
                            <li {...stylex.attrs(entry.depth > 2 && styles.nested)}>
                                <a
                                    {...stylex.attrs(
                                        styles.link,
                                        props.activeId() === entry.id && styles.active,
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

/// Return the active heading and a compact window around it.
function visibleContents(entries: readonly ContentsEntry[], activeId: string) {
    const maximumEntries = 13;

    // preserve short outlines in full
    if (entries.length <= maximumEntries) {
        return entries;
    }

    // center the current heading with slightly more lookahead
    const activeIndex = Math.max(0, entries.findIndex((entry) => entry.id === activeId));
    const start = Math.max(0, Math.min(activeIndex - 4, entries.length - maximumEntries));

    return entries.slice(start, start + maximumEntries);
}

const styles = stylex.create({
    active: {
        color: tokens.text,
        fontWeight: 600,
        boxShadow: `inset 2px 0 ${tokens.orange}`,
    },
    heading: {
        color: tokens.soft,
        fontFamily: tokens.monoFont,
        fontSize: "0.72rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        margin: "0 0 0.35rem",
        textTransform: "uppercase",
    },
    link: {
        display: "block",
        paddingLeft: "0.5rem",
        paddingBlock: "0.22rem",
        ":hover": {
            color: tokens.text,
        },
    },
    list: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    nested: {
        paddingLeft: "0.65rem",
    },
    root: {
        color: tokens.soft,
        display: "none",
        fontSize: "0.74rem",
        lineHeight: 1.4,
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        paddingTop: "1rem",
        "@media (min-width: 60rem)": {
            display: "block",
        },
    },
    rootMenu: {
        display: "block",
    },
});

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
