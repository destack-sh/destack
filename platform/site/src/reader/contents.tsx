import { type Accessor, createMemo, createSignal, For, onSettled, Show } from "@destack/view";
import * as stylex from "@destack/style";

import { publicationStyles } from "./publication.stylex";

/// One article heading in the rendered contents.
export type ContentsEntry = {
    /// The heading depth.
    depth: number;

    /// The heading identifier.
    id: string;

    /// The visible heading text.
    text: string;
};

/// One heading and its direct descendants.
type ContentsNode = ContentsEntry & {
    /// The headings nested directly below this heading.
    children: ContentsNode[];
};

/// Properties for an article heading tree.
type ContentsTreeProps = {
    /// The currently active heading identifier.
    activeId: Accessor<string>;

    /// The document headings.
    entries: readonly ContentsEntry[];

    /// Whether the root headings sit below a parent entry.
    isNested?: boolean;
};

/// Render the active article heading tree.
export function ContentsTree(props: ContentsTreeProps) {
    const nodes = createMemo(() => outlineFor(props.entries));

    return (
        <Show when={props.entries.length > 0}>
            <ContentsList activeId={props.activeId} isNested={props.isNested} nodes={nodes()} />
        </Show>
    );
}

/// Properties for one level of the article outline.
type ContentsListProps = {
    /// The currently active heading identifier.
    activeId: Accessor<string>;

    /// The headings at this outline level.
    nodes: readonly ContentsNode[];

    /// Whether this level is nested below another heading.
    isNested?: boolean;
};

/// Render one level of the article outline.
function ContentsList(props: ContentsListProps) {
    return (
        <ol {...stylex.attrs(styles.list, props.isNested && styles.nested)}>
            <For each={props.nodes}>
                {(node) => {
                    return (
                        <li>
                            <a
                                {...stylex.attrs(
                                    publicationStyles.collectionLink,
                                    props.activeId() === node.id && publicationStyles.active,
                                )}
                                href={`#${node.id}`}
                            >
                                <span>{node.text}</span>
                            </a>

                            <Show when={node.children.length > 0}>
                                <ContentsList
                                    activeId={props.activeId}
                                    isNested
                                    nodes={node.children}
                                />
                            </Show>
                        </li>
                    );
                }}
            </For>
        </ol>
    );
}

/// Build the authored heading hierarchy.
function outlineFor(entries: readonly ContentsEntry[]) {
    const roots: ContentsNode[] = [];
    const parents: ContentsNode[] = [];

    // attach each heading to the nearest preceding shallower heading
    for (const entry of entries) {
        let parent = parents.at(-1);
        while (parent != undefined && parent.depth >= entry.depth) {
            parents.pop();
            parent = parents.at(-1);
        }

        const node = { ...entry, children: [] } satisfies ContentsNode;
        if (parent == undefined) {
            roots.push(node);
        } else {
            parent.children.push(node);
        }
        parents.push(node);
    }

    return roots;
}

const styles = stylex.create({
    list: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    nested: {
        paddingLeft: "1rem",
    },
});

/// Track the last heading above the reading position.
export function trackActiveHeading(entries: readonly ContentsEntry[]) {
    const [activeId, setActiveId] = createSignal(entries[0]?.id ?? "");

    onSettled(() => {
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
        return () => {
            if (frame !== 0) {
                window.cancelAnimationFrame(frame);
            }

            window.removeEventListener("scroll", schedule);
            window.removeEventListener("resize", schedule);
        };
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
