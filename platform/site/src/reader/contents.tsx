import { createSignal, For, onCleanup, onMount, Show } from "solid-js";

export type ContentsEntry = {
    /// The heading depth.
    depth: number;

    /// The heading identifier.
    id: string;

    /// The visible heading text.
    text: string;
};

type ContentsProps = {
    /// The document headings.
    entries: readonly ContentsEntry[];
};

/// Render the active article outline on wide screens.
export function Contents(props: ContentsProps) {
    const activeId = activeHeading(props.entries);

    return (
        <Show when={props.entries.length > 0}>
            <nav aria-label="contents" class="contents">
                <p>[contents]</p>
                <ol>
                    <For each={props.entries}>
                        {(entry) => (
                            <li classList={{ "contents__nested": entry.depth > 2 }}>
                                <a
                                    classList={{ "contents__active": activeId() === entry.id }}
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
function activeHeading(entries: readonly ContentsEntry[]) {
    const [activeId, setActiveId] = createSignal(entries[0]?.id ?? "");

    onMount(() => {
        let frame = 0;

        // update at most once per rendered frame
        const update = () => {
            frame = 0;
            setActiveId(visibleHeading(entries));
        };

        const schedule = () => {
            if (frame === 0) {
                frame = window.requestAnimationFrame(update);
            }
        };

        update();
        window.addEventListener("scroll", schedule, { passive: true });
        window.addEventListener("resize", schedule);

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
    const offset = 96;
    let current = entries[0]?.id ?? "";

    for (const entry of entries) {
        const element = document.getElementById(entry.id);
        if (element == undefined) {
            continue;
        }

        if (element.getBoundingClientRect().top > offset) {
            break;
        }

        current = entry.id;
    }

    return current;
}
