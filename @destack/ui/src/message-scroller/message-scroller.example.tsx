import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { defineExample } from "@destack/package/declare";
import { createMemo, createSignal, For, type JSX, Show, useEnvironment } from "@destack/view";
import { Bubble, BubbleContent } from "../bubble/index.ts";
import { Button } from "../button/index.ts";
import { Cluster } from "../cluster/index.ts";
import {
    Composer,
    ComposerFooter,
    ComposerFrame,
    ComposerInput,
    ComposerSubmit,
} from "../composer/index.ts";
import { Marker, MarkerContent } from "../marker/index.ts";
import { Stack } from "../stack/index.ts";
import { Virtualizer } from "../virtualizer/index.ts";
import { Message, MessageContent } from "../message/index.ts";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerItem,
    MessageScrollerProvider,
    MessageScrollerViewport,
    useMessageScroller,
} from "./index.ts";

/** How often the assistant of the examples writes on, in milliseconds. */
const BEAT = 120;

/** The height a virtual row is taken to have until it is measured, in pixels. */
const ROW = 52;

/** The words an answer adds at a time. */
const WORDS = 4;

/** What the assistant of the examples answers. */
const ANSWER = Array.from(
    { length: 12 },
    (_, index) =>
        `${String(index + 1)}. Take the early ferry, as the light is best before ten and the piers are still empty. `,
).join("\n");

/** A message of the examples. */
interface Line {
    /** The message's id. */
    readonly id: string;
    /** Who wrote it. */
    readonly role: "user" | "assistant";
    /** What it says. */
    readonly text: string;
}

/** The styles of the examples' conversations. */
const styles = style.create({
    frame: {
        display: "flex",
        flexDirection: "column",
        height: "32rem",
        overflow: "hidden",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
    },
    virtual: (height: string) => ({
        position: "relative",
        height,
    }),
    virtualRow: (top: string) => ({
        position: "absolute",
        insetInline: 0,
        insetBlockStart: 0,
        paddingBlockEnd: space[4],
        transform: `translateY(${top})`,
    }),
    composer: {
        flexShrink: 0,
        padding: space[3],
    },
});

/** Write a conversation of some turns. */
function conversation(turns: number): readonly Line[] {
    return Array.from({ length: turns * 2 }, (_, index) => ({
        id: `line-${String(index)}`,
        role: index % 2 === 0 ? ("user" as const) : ("assistant" as const),
        text:
            index % 2 === 0
                ? `Question ${String(index / 2 + 1)}`
                : `Answer ${String((index + 1) / 2)}: it depends on the tide, which turns at noon.`,
    }));
}

/** Render one message: one's own in a bubble, the assistant's as text. */
function Row(properties: { readonly line: Line }): JSX.Element {
    return (
        <MessageScrollerItem
            messageId={properties.line.id}
            scrollAnchor={properties.line.role === "user"}
        >
            <Message align={properties.line.role === "user" ? "end" : "start"}>
                <MessageContent>
                    <Bubble variant={properties.line.role === "user" ? "secondary" : "ghost"}>
                        <BubbleContent>{properties.line.text}</BubbleContent>
                    </Bubble>
                </MessageContent>
            </Message>
        </MessageScrollerItem>
    );
}

/** Answer a line by line, and stop or finish. */
function createAssistant(initial: readonly Line[]) {
    // keep the lines, and the answer under way
    const { clock } = useEnvironment();
    const [lines, setLines] = createSignal(initial);
    const [timer, setTimer] = createSignal<number | undefined>(undefined, { ownedWrite: true });
    const stop = (): void => {
        clock.clearTimeout(timer());
        setTimer(undefined);
    };
    const write = (id: string, said: number): void => {
        // write some words on, until all are there
        const words = ANSWER.split(" ");
        const text = words.slice(0, said).join(" ");
        setLines((all) => all.map((line) => (line.id === id ? { ...line, text } : line)));
        setTimer(
            said >= words.length
                ? undefined
                : clock.setTimeout(() => write(id, said + WORDS), BEAT),
        );
    };
    const ask = (text: string): void => {
        // answer below the question
        const id = String(lines().length);
        setLines((all) => [
            ...all,
            { id: `line-${id}`, role: "user", text },
            { id: `answer-${id}`, role: "assistant", text: "" },
        ]);
        write(`answer-${id}`, WORDS);
    };
    const earlier = (count: number): void => {
        setLines((all) => [
            ...conversation(count).map((line) => ({
                ...line,
                id: `${String(all.length)}-${line.id}`,
            })),
            ...all,
        ]);
    };

    return { lines, isBusy: () => timer() !== undefined, ask, stop, earlier };
}

/** A conversation with an assistant, whose answers are written out. */
export const messageScrollerAssistant = defineExample({
    of: MessageScroller,
    name: "assistant",
    description: t`A conversation with an assistant: a question comes near the top, and its answer grows below it.`,
    layout: "padded",
    render: () => {
        const assistant = createAssistant(conversation(8));
        const [words, setWords] = createSignal("What should we see on Saturday?");

        return (
            <div {...style.attrs(styles.frame)}>
                <MessageScroller autoScroll defaultScrollPosition="last-anchor">
                    <MessageScrollerViewport>
                        <MessageScrollerContent aria-busy={assistant.isBusy() ? "true" : undefined}>
                            <For each={assistant.lines()} keyed={(line) => line.id}>
                                {(line) => <Row line={line()} />}
                            </For>
                        </MessageScrollerContent>
                    </MessageScrollerViewport>
                    <MessageScrollerButton />
                </MessageScroller>
                <div {...style.attrs(styles.composer)}>
                    <Composer
                        busy={assistant.isBusy()}
                        onStop={assistant.stop}
                        onSubmit={() => {
                            assistant.ask(words());
                            setWords("");
                        }}
                    >
                        <ComposerFrame>
                            <ComposerInput
                                placeholder="Ask anything"
                                value={words()}
                                onInput={(event) => setWords(event.currentTarget.value)}
                            />
                            <ComposerFooter>
                                <ComposerSubmit
                                    disabled={!assistant.isBusy() && words().trim() === ""}
                                />
                            </ComposerFooter>
                        </ComposerFrame>
                    </Composer>
                </div>
            </div>
        );
    },
});

/** Render buttons that go through a conversation from outside its frame. */
function Outline(): JSX.Element {
    const scroller = useMessageScroller();

    return (
        <Cluster space="2">
            <Button variant="outline" size="sm" onClick={() => scroller.scrollToMessage("line-8")}>
                Question 5
            </Button>
            <Button variant="outline" size="sm" onClick={() => scroller.scrollToStart()}>
                First
            </Button>
            <span data-slot="current">{scroller.currentAnchorId()}</span>
        </Cluster>
    );
}

/** Estimate the height of a virtual row. */
function estimateRow(): number {
    return ROW;
}

/** Render only the rows in view of a conversation, on its own scroller. */
function VirtualRows(properties: { readonly lines: readonly Line[] }): JSX.Element {
    // measure the rows in the conversation's view
    const scroller = useMessageScroller();
    const keys = createMemo(() => {
        const { lines } = properties;

        return (index: number) => lines[index]?.id ?? index;
    });
    const virtualizer = new Virtualizer({
        count: () => properties.lines.length,
        estimateSize: () => estimateRow,
        scrollElement: scroller.viewport,
        itemKey: keys,
        anchorTo: "end",
        followOnAppend: true,
    });

    return (
        <div {...style.attrs(styles.virtual(`${String(virtualizer.totalSize())}px`))}>
            <For each={virtualizer.items()} keyed={(item) => item.key}>
                {(item) => (
                    <div
                        {...virtualizer.itemAttributes(item())}
                        {...style.attrs(styles.virtualRow(`${String(item().start)}px`))}
                    >
                        <Bubble variant={item().index % 2 === 0 ? "secondary" : "ghost"}>
                            <BubbleContent>{properties.lines[item().index]?.text}</BubbleContent>
                        </Bubble>
                    </div>
                )}
            </For>
        </div>
    );
}

/** Ten thousand messages, only those in view in the document. */
export const messageScrollerVirtual = defineExample({
    of: MessageScroller,
    name: "virtual",
    description: t`Ten thousand messages, only those in view in the document.`,
    layout: "padded",
    render: () => {
        const [lines, setLines] = createSignal(conversation(5_000));

        return (
            <Stack space="3">
                <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                        setLines((all) => [
                            ...all,
                            { id: `line-${String(all.length)}`, role: "user", text: "One more" },
                        ])
                    }
                >
                    Send a message
                </Button>
                <div {...style.attrs(styles.frame)}>
                    <MessageScroller autoScroll>
                        <MessageScrollerViewport>
                            <MessageScrollerContent>
                                <VirtualRows lines={lines()} />
                            </MessageScrollerContent>
                        </MessageScrollerViewport>
                        <MessageScrollerButton />
                    </MessageScroller>
                </div>
            </Stack>
        );
    },
});

/** A conversation with an outline beside it, which loads what came before. */
export const messageScrollerOutline = defineExample({
    of: MessageScroller,
    name: "outline",
    description: t`A conversation with buttons outside its frame, and a row that loads what came before.`,
    layout: "padded",
    render: () => {
        const assistant = createAssistant(conversation(20));
        const [isWhole, setWhole] = createSignal(false);

        return (
            <MessageScrollerProvider>
                <Outline />
                <div {...style.attrs(styles.frame)}>
                    <MessageScroller>
                        <MessageScrollerViewport>
                            <MessageScrollerContent>
                                <MessageScrollerItem>
                                    <Show
                                        when={!isWhole()}
                                        fallback={
                                            <Marker variant="separator">
                                                <MarkerContent>The beginning</MarkerContent>
                                            </Marker>
                                        }
                                    >
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={() => {
                                                assistant.earlier(6);
                                                setWhole(true);
                                            }}
                                        >
                                            Show earlier messages
                                        </Button>
                                    </Show>
                                </MessageScrollerItem>
                                <For each={assistant.lines()} keyed={(line) => line.id}>
                                    {(line) => <Row line={line()} />}
                                </For>
                            </MessageScrollerContent>
                        </MessageScrollerViewport>
                        <MessageScrollerButton direction="start" />
                        <MessageScrollerButton />
                    </MessageScroller>
                </div>
            </MessageScrollerProvider>
        );
    },
});
