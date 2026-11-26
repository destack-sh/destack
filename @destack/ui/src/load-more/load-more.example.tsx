import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { defineExample } from "@destack/package/declare";
import { createSignal, For, useEnvironment } from "@destack/view";
import { Spinner } from "../spinner/index.ts";
import { LoadMore } from "./load-more.tsx";

/** The notes a page has. */
const PAGE = 20;

/** The styles of the example's list. */
const styles = style.create({
    list: {
        width: "100%",
        blockSize: "16rem",
        overflowY: "auto",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
    },
    row: {
        paddingInline: space[3],
        paddingBlock: space[2],
    },
    more: {
        padding: space[3],
    },
});

/** A list of notes that loads the next page as its end scrolls near. */
export const loadMoreNotes = defineExample({
    of: LoadMore,
    name: "notes",
    description: t`A list of notes that loads the next page as its end scrolls near.`,
    properties: {
        scrollOffset: 1,
    },
    render: (properties) => {
        // add a page a moment after each request, five pages in all
        const { clock } = useEnvironment();
        const [count, setCount] = createSignal(PAGE);
        const [isLoading, setLoading] = createSignal(false);
        const load = () => {
            setLoading(true);
            clock.setTimeout(() => {
                setCount((current) => current + PAGE);
                setLoading(false);
            }, 600);
        };

        return (
            <div tabindex="0" {...style.attrs(text.sm, styles.list)}>
                <div role="list" aria-label="Notes">
                    <For each={Array.from({ length: count() }, (_, index) => index + 1)}>
                        {(note) => (
                            <div role="listitem" {...style.attrs(styles.row)}>
                                Note {note}
                            </div>
                        )}
                    </For>
                </div>
                {count() < 5 * PAGE ? (
                    <LoadMore
                        scrollOffset={properties.scrollOffset ?? 1}
                        onLoadMore={load}
                        isLoading={isLoading()}
                        xstyle={styles.more}
                    >
                        <Spinner aria-label="Loading notes" />
                    </LoadMore>
                ) : null}
            </div>
        );
    },
});

/** A list of notes while its next page loads. */
export const loadMoreNotesLoading = defineExample({
    of: LoadMore,
    name: "notesLoading",
    description: t`A list of notes while its next page loads.`,
    properties: {
        isLoading: true,
    },
    render: (properties) => (
        <div tabindex="0" {...style.attrs(text.sm, styles.list)}>
            <div role="list" aria-label="Notes">
                <For each={[1, 2, 3]}>
                    {(note) => (
                        <div role="listitem" {...style.attrs(styles.row)}>
                            Note {note}
                        </div>
                    )}
                </For>
            </div>
            <LoadMore
                isLoading={properties.isLoading ?? true}
                onLoadMore={() => undefined}
                xstyle={styles.more}
            >
                <Spinner aria-label="Loading notes" />
            </LoadMore>
        </div>
    ),
});
