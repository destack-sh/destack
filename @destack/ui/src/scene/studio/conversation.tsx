import hash from "@destack/icon/phosphor/hash";
import quotes from "@destack/icon/phosphor/quotes";
import x from "@destack/icon/phosphor/x";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import {
    createEffect,
    createMemo,
    createSignal,
    For,
    type JSX,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import { Badge } from "../../badge/index.ts";
import { Button } from "../../button/index.ts";
import { Cluster } from "../../cluster/index.ts";
import {
    Composer,
    ComposerFooter,
    ComposerFrame,
    ComposerHeader,
    ComposerInput,
    ComposerSubmit,
} from "../../composer/index.ts";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "../../empty/index.ts";
import { Icon } from "../../icon/index.ts";
import { LoadMore } from "../../load-more/index.ts";
import { Marker, MarkerContent } from "../../marker/index.ts";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerProvider,
    MessageScrollerViewport,
    useMessageScroller,
} from "../../message-scroller/index.ts";
import { Popover, PopoverContent } from "../../popover/index.ts";
import type { AnchorBox } from "../../position/index.ts";
import { Quote, QuoteContent, QuoteDismiss, QuoteIcon } from "../../quote/index.ts";
import { SidebarTrigger } from "../../sidebar/index.ts";
import type { Assistant } from "./assistant.ts";
import { type Conversation, type Chat, ME, type ChatMessage, type Row, rowsOf } from "./chat.ts";
import { isBusy, MessageView, nameOf } from "./message.tsx";

/** The messages a conversation draws at first, and how many more near its start. */
const PAGE = 60;

/** What a person may ask an assistant first. */
const SUGGESTIONS = [
    "Compare the two quotes for the flyer",
    "Mail Nordpapier that we order on Friday",
    "Say what is left before the fair",
];

/** What a message is answered with: the message, and some of its words. */
interface Quoted {
    /** The message's id. */
    readonly message: string;
    /** The words, undefined for all of them. */
    readonly excerpt: string | undefined;
}

/** The messages a conversation draws, from one place to another or to its end. */
interface Window {
    /** The place of the first. */
    readonly start: number;
    /** The place after the last, undefined up to the end. */
    readonly end: number | undefined;
}

/** Words a person selected in a message, and where they stand. */
interface Selected extends Quoted {
    /** The words. */
    readonly excerpt: string;
    /** Where they stand in the window. */
    readonly box: AnchorBox;
}

/** The styles of a conversation. */
const styles = style.create({
    screen: {
        display: "flex",
        flexGrow: 1,
        minHeight: 0,
        position: "relative",
    },
    column: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        flexBasis: 0,
        minWidth: 0,
        minHeight: 0,
    },
    thread: {
        flexGrow: 0,
        flexBasis: { default: "24rem", "@media (max-width: 64rem)": "auto" },
        position: { default: "static", "@media (max-width: 64rem)": "absolute" },
        inset: 0,
        backgroundColor: color.background,
        borderInlineStartWidth: {
            default: stroke.border,
            "@media (max-width: 64rem)": 0,
        },
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
    },
    bar: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        columnGap: space[2],
        minHeight: `calc(${size[2]} + ${space[2]})`,
        paddingInline: space[3],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    title: {
        margin: 0,
        fontWeight: weight.semibold,
        whiteSpace: "nowrap",
    },
    topic: {
        flexGrow: 1,
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        color: color.mutedForeground,
    },
    log: {
        flexGrow: 1,
        minHeight: 0,
    },
    rows: {
        boxSizing: "border-box",
        width: "100%",
        paddingInline: space[4],
        paddingBlock: space[4],
    },
    narrow: {
        maxWidth: "46rem",
        marginInline: "auto",
    },
    composer: {
        flexShrink: 0,
        boxSizing: "border-box",
        width: "100%",
        paddingInline: space[4],
        paddingBlockEnd: space[4],
    },
    start: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        justifyContent: "center",
        rowGap: space[5],
        minHeight: 0,
    },
});

/** Render a conversation, with the thread a person opened beside it. */
export function ConversationPage(properties: {
    /** The chat. */
    readonly chat: Chat;
    /** The assistant. */
    readonly assistant: Assistant;
    /** The conversation. */
    readonly conversation: Conversation;
}): JSX.Element {
    const [root, setRoot] = createSignal<string>();

    return (
        <div {...style.attrs(text.sm, styles.screen)}>
            <div {...style.attrs(styles.column)}>
                <Header conversation={properties.conversation} />
                <Log
                    chat={properties.chat}
                    assistant={properties.assistant}
                    conversation={properties.conversation}
                    onThread={(id) => void setRoot(id)}
                />
            </div>
            <Show when={root()}>
                {(open) => (
                    <aside aria-label="Thread" {...style.attrs(styles.column, styles.thread)}>
                        <div {...style.attrs(styles.bar)}>
                            <h2 {...style.attrs(text.sm, styles.title)}>Thread</h2>
                            <span {...style.attrs(styles.topic)}>
                                {properties.conversation.name}
                            </span>
                            <Button
                                size="icon-sm"
                                variant="ghost"
                                aria-label="Close the thread"
                                onClick={() => setRoot(undefined)}
                            >
                                <Icon icon={x} />
                            </Button>
                        </div>
                        <For each={[open()]}>
                            {(id) => (
                                <Log
                                    chat={properties.chat}
                                    assistant={properties.assistant}
                                    conversation={properties.conversation}
                                    root={id}
                                />
                            )}
                        </For>
                    </aside>
                )}
            </Show>
        </div>
    );
}

/** Render where the person is. */
function Header(properties: {
    /** The conversation. */
    readonly conversation: Conversation;
}): JSX.Element {
    const pinned = () => properties.conversation.messages.filter(({ isPinned }) => isPinned).length;

    return (
        <header {...style.attrs(styles.bar)}>
            <SidebarTrigger />
            <Show when={properties.conversation.kind === "channel"}>
                <Icon icon={hash} />
            </Show>
            <h1 {...style.attrs(text.sm, styles.title)}>{properties.conversation.name}</h1>
            <span {...style.attrs(styles.topic)}>{properties.conversation.topic}</span>
            <Show when={pinned() > 0}>
                <Badge variant="secondary">{pinned()} pinned</Badge>
            </Show>
        </header>
    );
}

/** Render the messages of a conversation or a thread, and the box to write in. */
function Log(properties: {
    /** The chat. */
    readonly chat: Chat;
    /** The assistant. */
    readonly assistant: Assistant;
    /** The conversation. */
    readonly conversation: Conversation;
    /** The message whose thread it is, absent for the conversation itself. */
    readonly root?: string;
    /** Open a message's thread. */
    readonly onThread?: (message: string) => void;
}): JSX.Element {
    // read the messages, and what is under each
    const { chat, assistant } = properties;
    const root = untrack(() => properties.root);
    const id = untrack(() => properties.conversation.id);
    const isAssistant = untrack(() => properties.conversation.kind) === "assistant";
    const all = createMemo(() => (root === undefined ? chat.main(id) : chat.thread(id, root)));
    const replies = createMemo(() =>
        Map.groupBy(properties.conversation.messages, ({ thread }) => thread),
    );

    // draw the last, more near either end, and those around a message the person goes to
    const [drawn, setDrawn] = createSignal<Window>(
        untrack(() => ({ start: Math.max(0, all().length - PAGE), end: undefined })),
        { ownedWrite: true },
    );
    const read = untrack(() => properties.conversation.read);
    const rows = createMemo(() => {
        const { start, end } = drawn();

        return rowsOf(all().slice(start, end), root === undefined ? read : Infinity);
    });
    const reveal = (message: string): void => {
        // draw the messages around one that is not drawn
        const at = all().findIndex((each) => each.id === message);
        const { start, end = all().length } = drawn();
        if (at !== -1 && (at < start || at >= end)) {
            const from = Math.max(0, at - PAGE / 2);
            const to = from + PAGE;
            setDrawn({ start: from, end: to >= all().length ? undefined : to });
        }
    };
    const [quote, setQuote] = createSignal<Quoted>();
    const [selected, setSelected] = createSignal<Selected>();
    const select = (event: { readonly currentTarget: HTMLElement }): void => {
        // offer to quote words selected within one message
        const selection = event.currentTarget.ownerDocument.getSelection();
        const range =
            selection === null || selection.isCollapsed ? undefined : selection.getRangeAt(0);
        const row =
            range?.commonAncestorContainer.parentElement?.closest<HTMLElement>("[data-message-id]");
        const message = row?.dataset["messageId"];
        const excerpt = range?.toString().trim() ?? "";
        setSelected(
            range === undefined || message === undefined || excerpt === ""
                ? undefined
                : { message, excerpt, box: range.getBoundingClientRect() },
        );
    };
    const busy = () => all().find(isBusy);
    createEffect(
        () => selected() !== undefined,
        (isOpen) => {
            // let go of the words as they scroll away or the selection goes
            if (!isOpen) {
                return undefined;
            }
            const leaving = new AbortController();
            const options = { capture: true, passive: true, signal: leaving.signal };
            document.addEventListener("scroll", () => setSelected(undefined), options);
            document.addEventListener(
                "selectionchange",
                () => {
                    if (document.getSelection()?.isCollapsed !== false) {
                        setSelected(undefined);
                    }
                },
                options,
            );

            return () => leaving.abort();
        },
    );
    const send = (written: string): void => {
        // say it at the end, drawn, and let the assistant answer where it is asked
        chat.say(id, {
            author: ME,
            text: written,
            thread: root,
            quote: quote()?.message,
            excerpt: quote()?.excerpt,
        });
        setQuote(undefined);
        setDrawn({ start: Math.max(0, all().length - PAGE), end: undefined });
        if (isAssistant || /@assistant\b/iu.test(written)) {
            assistant.answer(id, written, root);
        }
    };

    return (
        <MessageScrollerProvider autoScroll={!isAssistant} defaultScrollPosition="end">
            <Show
                when={all().length > 0}
                fallback={
                    <div {...style.attrs(styles.start)}>
                        <Empty>
                            <EmptyHeader>
                                <EmptyTitle>
                                    {isAssistant ? "What should we look at?" : "Nothing here yet"}
                                </EmptyTitle>
                                <EmptyDescription>
                                    {isAssistant
                                        ? "The assistant reads the studio’s files and asks before it sends anything."
                                        : "Say something to begin."}
                                </EmptyDescription>
                            </EmptyHeader>
                            <Show when={isAssistant}>
                                <Cluster space="2" justify="center">
                                    <For each={SUGGESTIONS}>
                                        {(each) => (
                                            <Button
                                                size="sm"
                                                variant="outline"
                                                onClick={() => send(each)}
                                            >
                                                {each}
                                            </Button>
                                        )}
                                    </For>
                                </Cluster>
                            </Show>
                        </Empty>
                    </div>
                }
            >
                <MessageScroller xstyle={styles.log}>
                    <MessageScrollerViewport>
                        <MessageScrollerContent
                            onClick={select}
                            onKeyUp={select}
                            aria-busy={busy() === undefined ? undefined : "true"}
                            xstyle={[styles.rows, isAssistant && styles.narrow]}
                        >
                            <Show when={drawn().start > 0}>
                                <LoadMore
                                    onLoadMore={() =>
                                        setDrawn(({ start, end }) => ({
                                            start: Math.max(0, start - PAGE),
                                            end,
                                        }))
                                    }
                                />
                            </Show>
                            <For each={rows()} keyed={(row) => row.key}>
                                {(row) => (
                                    <RowView
                                        chat={chat}
                                        assistant={assistant}
                                        conversation={id}
                                        row={row()}
                                        isAssistant={isAssistant}
                                        replies={root === undefined ? replies() : undefined}
                                        onThread={properties.onThread}
                                        onQuote={(message) =>
                                            void setQuote({ message, excerpt: undefined })
                                        }
                                        onReveal={reveal}
                                    />
                                )}
                            </For>
                            <Show when={drawn().end}>
                                {(end) => (
                                    <LoadMore
                                        onLoadMore={() => {
                                            // draw the later ones, up to the last
                                            const to = end() + PAGE;
                                            setDrawn(({ start }) => ({
                                                start,
                                                end: to >= all().length ? undefined : to,
                                            }));
                                        }}
                                    />
                                )}
                            </Show>
                        </MessageScrollerContent>
                    </MessageScrollerViewport>
                    <MessageScrollerButton />
                    <Popover
                        open={selected() !== undefined}
                        onOpenChange={(isOpen) => {
                            if (!isOpen) {
                                setSelected(undefined);
                            }
                        }}
                    >
                        <PopoverContent anchor={selected()?.box} side="top" width="fit">
                            <Button
                                size="sm"
                                variant="ghost"
                                onPointerDown={(event) => event.preventDefault()}
                                onClick={() => {
                                    // answer the words, and leave the selection
                                    setQuote(selected());
                                    setSelected(undefined);
                                    document.getSelection()?.removeAllRanges();
                                }}
                            >
                                <Icon icon={quotes} />
                                Quote
                            </Button>
                        </PopoverContent>
                    </Popover>
                </MessageScroller>
            </Show>
            <div {...style.attrs(styles.composer, isAssistant && styles.narrow)}>
                <Write
                    placeholder={placeholderOf(properties.conversation, root)}
                    draft={root === undefined ? properties.conversation.draft : undefined}
                    quoted={quote()}
                    message={chat.message(quote()?.message ?? "")}
                    isBusy={isAssistant && busy() !== undefined}
                    onDraft={(draft) => chat.update(id, { draft })}
                    onUnquote={() => setQuote(undefined)}
                    onStop={() => {
                        const message = busy();
                        if (message !== undefined) {
                            assistant.stop(message.id);
                        }
                    }}
                    onSend={send}
                />
            </div>
        </MessageScrollerProvider>
    );
}

/** Render a row: a day, where the unread begin, or a message. */
function RowView(properties: {
    /** The chat. */
    readonly chat: Chat;
    /** The assistant. */
    readonly assistant: Assistant;
    /** The conversation. */
    readonly conversation: string;
    /** The row. */
    readonly row: Row;
    /** Whether it is a conversation with the assistant alone. */
    readonly isAssistant: boolean;
    /** The messages under each, undefined in a thread. */
    readonly replies: ReadonlyMap<string | undefined, readonly ChatMessage[]> | undefined;
    /** Open a message's thread. */
    readonly onThread?: ((message: string) => void) | undefined;
    /** Answer a message with its words. */
    readonly onQuote: (message: string) => void;
    /** Draw a message that is not drawn. */
    readonly onReveal: (message: string) => void;
}): JSX.Element {
    // draw a row by its kind, which its key keeps
    const locale = useLocale();
    const scroller = useMessageScroller();
    const kind = untrack(() => properties.row.kind);
    const message = () => (properties.row.kind === "message" ? properties.row : undefined);
    const id = untrack(() => message()?.message.id ?? "");
    const label = () =>
        properties.row.kind === "day"
            ? locale.date(properties.row.at, { weekday: "long", day: "numeric", month: "long" })
            : "New";
    if (kind !== "message") {
        return (
            <Marker variant="separator">
                <MarkerContent>{label()}</MarkerContent>
            </Marker>
        );
    }

    return (
        <Show when={message()}>
            {(row) => (
                <MessageView
                    chat={properties.chat}
                    assistant={properties.assistant}
                    conversation={properties.conversation}
                    message={row().message}
                    isFollowing={row().isFollowing}
                    isTurn={properties.isAssistant && row().message.author === ME}
                    isAssistant={properties.isAssistant}
                    replies={properties.replies?.get(id)}
                    onThread={
                        properties.onThread === undefined || properties.isAssistant
                            ? undefined
                            : () => properties.onThread?.(id)
                    }
                    onQuote={() => properties.onQuote(id)}
                    onGo={(target) => {
                        // draw it where it is not, and go there once it is
                        properties.onReveal(target);
                        scroller.scrollToMessage(target, { align: "center", behavior: "smooth" });
                    }}
                />
            )}
        </Show>
    );
}

/** Render the box a message is written in. */
function Write(properties: {
    /** What it asks for. */
    readonly placeholder: string;
    /** What the person wrote before, undefined where it is not kept. */
    readonly draft: string | undefined;
    /** What it answers, undefined for nothing. */
    readonly quoted: Quoted | undefined;
    /** The message it answers, undefined for none. */
    readonly message: ChatMessage | undefined;
    /** Whether the assistant answers. */
    readonly isBusy: boolean;
    /** Keep what is written. */
    readonly onDraft: (draft: string) => void;
    /** Answer nothing. */
    readonly onUnquote: () => void;
    /** Stop the assistant. */
    readonly onStop: () => void;
    /** Send what is written. */
    readonly onSend: (written: string) => void;
}): JSX.Element {
    // keep what is written with the conversation
    const [words, setWords] = createSignal(untrack(() => properties.draft ?? ""));
    const [input, setInput] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const write = (written: string): void => {
        setWords(written);
        if (properties.draft !== undefined) {
            properties.onDraft(written);
        }
    };
    createEffect(
        () => properties.quoted,
        (quoted) => {
            if (quoted !== undefined) {
                untrack(input)?.focus();
            }
        },
    );

    return (
        <Composer
            busy={properties.isBusy}
            onStop={() => properties.onStop()}
            onSubmit={() => {
                properties.onSend(words().trim());
                write("");
            }}
        >
            <ComposerFrame size="sm">
                <Show when={properties.message}>
                    {(quoted) => (
                        <ComposerHeader>
                            <Quote>
                                <QuoteIcon />
                                <QuoteContent>
                                    {nameOf(quoted().author)}:{" "}
                                    {properties.quoted?.excerpt ??
                                        quoted().answers[quoted().shown]?.text ??
                                        quoted().text}
                                </QuoteContent>
                                <QuoteDismiss onClick={() => properties.onUnquote()} />
                            </Quote>
                        </ComposerHeader>
                    )}
                </Show>
                <ComposerInput
                    aria-label={properties.placeholder}
                    placeholder={properties.placeholder}
                    ref={(element) => setInput(element)}
                    value={words()}
                    onInput={(event) => write(event.currentTarget.value)}
                />
                <ComposerFooter>
                    <ComposerSubmit
                        size="icon-sm"
                        disabled={!properties.isBusy && words().trim() === ""}
                    />
                </ComposerFooter>
            </ComposerFrame>
        </Composer>
    );
}

/** Say what a box asks for. */
function placeholderOf(conversation: Conversation, root: string | undefined): string {
    if (root !== undefined) {
        return "Reply…";
    } else if (conversation.kind === "assistant") {
        return "Ask anything";
    }

    return `Message ${conversation.kind === "channel" ? "#" : ""}${conversation.name}`;
}
