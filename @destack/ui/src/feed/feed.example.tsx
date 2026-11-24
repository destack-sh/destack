import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { defineExample } from "@destack/package/declare";
import { createSignal, useEnvironment } from "@destack/view";
import { Button } from "../button/index.ts";
import { Spinner } from "../spinner/index.ts";
import { Stack } from "../stack/index.ts";
import { Feed } from "./feed.tsx";

/** The entries a page has. */
const PAGE = 30;

/** An entry of the examples' feeds. */
interface Entry {
    /** The entry's number, later ones higher. */
    readonly id: number;
    /** The entry's text. */
    readonly text: string;
}

/** The styles of the examples' feeds. */
const styles = style.create({
    feed: {
        blockSize: "20rem",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
    },
    entry: {
        paddingInline: space[3],
        paddingBlock: space[3],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    muted: {
        color: color.mutedForeground,
    },
});

/** Write an entry of a space's activity. */
function edit(id: number): string {
    return `Ada edited note ${String(id)}`;
}

/** Write a message of a conversation. */
function message(id: number): string {
    return `Message ${String(id)}`;
}

/** Write a page of entries from a number on. */
function entries(from: number, write: (id: number) => string): readonly Entry[] {
    return Array.from({ length: PAGE }, (_, index) => ({
        id: from + index,
        text: write(from + index),
    }));
}

/** A space's activity that loads later entries as its end scrolls near. */
export const feedActivity = defineExample({
    of: Feed,
    layout: "padded",
    name: "activity",
    description: "a space's activity that loads later entries as its end scrolls near",
    properties: {
        "aria-label": "Activity",
    },
    render: (properties) => {
        // add a page a moment after each request
        const { clock } = useEnvironment();
        const [each, setEach] = createSignal(entries(1, edit));
        const [isLoading, setLoading] = createSignal(false);
        const load = () => {
            setLoading(true);
            clock.setTimeout(() => {
                setEach((current) => [...current, ...entries(current.length + 1, edit)]);
                setLoading(false);
            }, 600);
        };

        return (
            <Feed
                each={each()}
                label={(entry) => entry.text}
                itemKey={(entry) => entry.id}
                estimateSize={() => 49}
                onLoadMore={load}
                isLoading={isLoading()}
                loading={<Spinner aria-label="Loading activity" />}
                aria-label={properties["aria-label"]}
                xstyle={styles.feed}
            >
                {(entry) => <div {...style.attrs(text.sm, styles.entry)}>{entry.text}</div>}
            </Feed>
        );
    },
});

/** A conversation that starts at its newest message and loads older ones above. */
export const feedConversation = defineExample({
    of: Feed,
    layout: "padded",
    name: "conversation",
    description:
        "a conversation that starts at its newest message, loads older ones above and follows new ones",
    render: () => {
        // start with the newest page, numbered so that older messages come lower
        const { clock } = useEnvironment();
        const [each, setEach] = createSignal(entries(1000, message));
        const [isLoading, setLoading] = createSignal(false);
        const loadOlder = () => {
            setLoading(true);
            clock.setTimeout(() => {
                setEach((current) => [
                    ...entries((current[0]?.id ?? PAGE) - PAGE, message),
                    ...current,
                ]);
                setLoading(false);
            }, 600);
        };
        const send = () =>
            setEach((current) => {
                const id = (current.at(-1)?.id ?? 0) + 1;

                return [...current, { id, text: message(id) }];
            });

        return (
            <Stack space="2">
                <Feed
                    each={each()}
                    label={(entry) => entry.text}
                    itemKey={(entry) => entry.id}
                    estimateSize={() => 49}
                    anchorTo="end"
                    followOnAppend
                    onLoadMore={loadOlder}
                    isLoading={isLoading()}
                    loading={<Spinner aria-label="Loading older messages" />}
                    aria-label="Conversation"
                    xstyle={styles.feed}
                >
                    {(entry) => <div {...style.attrs(text.sm, styles.entry)}>{entry.text}</div>}
                </Feed>
                <Button onClick={send}>Send a message</Button>
            </Stack>
        );
    },
});
