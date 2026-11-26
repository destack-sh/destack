import arrowBendUpLeft from "@destack/icon/phosphor/arrow-bend-up-left";
import arrowsClockwise from "@destack/icon/phosphor/arrows-clockwise";
import dotsThree from "@destack/icon/phosphor/dots-three";
import pushPin from "@destack/icon/phosphor/push-pin";
import smiley from "@destack/icon/phosphor/smiley";
import sparkle from "@destack/icon/phosphor/sparkle";
import { Place } from "@destack/object";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space } from "@destack/theme/tokens.stylex";
import {
    createSignal,
    For,
    handle,
    type JSX,
    provide,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import { ActionButton, MenuActionList } from "../../action/index.ts";
import { Avatar, AvatarFallback } from "../../avatar/index.ts";
import { Bubble, BubbleContent } from "../../bubble/index.ts";
import { Button } from "../../button/index.ts";
import { CopyButton } from "../../copy-button/index.ts";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuTrigger,
} from "../../dropdown-menu/index.ts";
import { Icon } from "../../icon/index.ts";
import { useMarkdown } from "../../markdown/index.ts";
import {
    Message,
    MessageAuthor,
    MessageAvatar,
    MessageContent,
    MessageFooter,
    MessageHeader,
    MessageToolbar,
} from "../../message/index.ts";
import { MessageScrollerItem } from "../../message-scroller/index.ts";
import { Quote, QuoteContent, QuoteIcon } from "../../quote/index.ts";
import { Reaction, ReactionGroup } from "../../reaction/index.ts";
import { RelativeTime } from "../../relative-time/index.ts";
import { RichText } from "../../rich-text/index.ts";
import { ToolbarButton } from "../../toolbar/index.ts";
import { AnswerPicker, AnswerView } from "./answer.tsx";
import type { Assistant } from "./assistant.ts";
import {
    copyMessage,
    deleteMessage,
    editMessage,
    markUnread,
    pinMessage,
    quoteMessage,
    reactToMessage,
    replyInThread,
    retryAnswer,
} from "./chat.example.ts";
import { CommentBox } from "./comment.tsx";
import { ASSISTANT, type Chat, ME, type ChatMessage, referenceOfMessage } from "./chat.ts";
import { Face } from "./shell.tsx";
import { personOf } from "./state.ts";

/** The reactions people pick from. */
const EMOJIS = ["👍", "❤️", "🎉", "👀", "🙏"];

/** The styles of messages. */
const styles = style.create({
    muted: {
        color: color.mutedForeground,
    },
    quote: {
        marginBlockEnd: space[1],
    },
    following: {
        marginBlockStart: `calc(${space[1]} - ${space[6]})`,
    },
});

/** Read the name of a member. */
export function nameOf(author: string): string {
    return author === ASSISTANT ? "Assistant" : (personOf(author)?.name ?? "Somebody");
}

/** Render a member's picture. */
export function MemberFace(properties: {
    /** The member. */
    readonly author: string;
}): JSX.Element {
    return (
        <Show
            when={personOf(properties.author)}
            fallback={
                <Avatar size="sm">
                    <AvatarFallback>
                        <Icon icon={sparkle} />
                    </AvatarFallback>
                </Avatar>
            }
        >
            {(person) => <Face person={person()} />}
        </Show>
    );
}

/** Render what a member said, with what a person does with it. */
export function MessageView(properties: {
    /** The chat. */
    readonly chat: Chat;
    /** The assistant. */
    readonly assistant: Assistant;
    /** The conversation it is in. */
    readonly conversation: string;
    /** The message. */
    readonly message: ChatMessage;
    /** Whether it stands under another of the same person. */
    readonly isFollowing: boolean;
    /** Whether it is a conversation with the assistant alone. */
    readonly isAssistant: boolean;
    /** The messages under it, undefined in a thread. */
    readonly replies: readonly ChatMessage[] | undefined;
    /** Open the thread under it, undefined where it stands in one. */
    readonly onThread: (() => void) | undefined;
    /** Answer it with its words. */
    readonly onQuote: () => void;
    /** Go to a message. */
    readonly onGo: (message: string) => void;
    /** Whether it starts a turn, which comes near the top as it is sent. */
    readonly isTurn: boolean;
}): JSX.Element {
    // follow the message
    const { chat, assistant } = properties;
    const [isEditing, setEditing] = createSignal(false);
    const message = () => properties.message;
    const isOwn = () => message().author === ME;
    const isBubble = () => properties.isAssistant && isOwn();
    const answer = () => message().answers[message().shown];
    const words = () => answer()?.text ?? message().text;
    const question = () => questionOf(chat, properties.conversation, message());

    return (
        <MessageScrollerItem
            messageId={untrack(() => properties.message.id)}
            scrollAnchor={properties.isTurn}
            xstyle={properties.isFollowing && styles.following}
            ref={[
                provide(() => Place.current(referenceOfMessage(message().id))),
                handle(reactToMessage, {
                    choices: () => EMOJIS.map((emoji) => ({ value: emoji, title: emoji })),
                    chosen: () =>
                        Object.entries(message().reactions).flatMap(([emoji, people]) =>
                            people.includes(ME) ? [emoji] : [],
                        ),
                    run: ({ values }) => chat.react(message().id, values.emoji),
                }),
                handle(replyInThread, {
                    isEnabled: () => properties.onThread !== undefined,
                    run: () => properties.onThread?.(),
                }),
                handle(quoteMessage, { run: () => properties.onQuote() }),
                handle(editMessage, { isEnabled: isOwn, run: () => void setEditing(true) }),
                handle(copyMessage, { run: () => navigator.clipboard.writeText(words()) }),
                handle(pinMessage, {
                    isOn: () => message().isPinned,
                    run: () =>
                        chat.change(message().id, ({ isPinned }) => ({
                            isPinned: !isPinned,
                        })),
                }),
                handle(markUnread, {
                    isEnabled: () => !properties.isAssistant,
                    run: () => chat.unreadFrom(properties.conversation, message().id),
                }),
                handle(retryAnswer, {
                    isEnabled: () => answer() !== undefined && !isBusy(message()),
                    run: () => assistant.retry(message().id, question()),
                }),
                handle(deleteMessage, {
                    isEnabled: isOwn,
                    run: () => chat.remove(message().id),
                }),
            ]}
        >
            <Message align={isBubble() ? "end" : "start"}>
                <Show when={!properties.isAssistant}>
                    <MessageAvatar position="top">
                        <Show when={!properties.isFollowing}>
                            <MemberFace author={message().author} />
                        </Show>
                    </MessageAvatar>
                </Show>
                <MessageContent>
                    <Show when={!properties.isAssistant && !properties.isFollowing}>
                        <MessageHeader>
                            <MessageAuthor>{nameOf(message().author)}</MessageAuthor>
                            <RelativeTime value={message().at} />
                            <Show when={message().isPinned}>
                                <Icon icon={pushPin} />
                            </Show>
                        </MessageHeader>
                    </Show>
                    <Show when={message().quote}>
                        {(quoted) => (
                            <QuoteOf
                                message={chat.message(quoted())}
                                excerpt={message().excerpt}
                                onGo={() => properties.onGo(quoted())}
                            />
                        )}
                    </Show>
                    <Show
                        when={isEditing()}
                        fallback={
                            <Show
                                when={answer()}
                                fallback={
                                    <Show
                                        when={isBubble()}
                                        fallback={<Text text={message().text} />}
                                    >
                                        <Bubble variant="secondary">
                                            <BubbleContent>
                                                <Text text={message().text} />
                                            </BubbleContent>
                                        </Bubble>
                                    </Show>
                                }
                            >
                                {(shown) => (
                                    <AnswerView
                                        chat={chat}
                                        assistant={assistant}
                                        message={message()}
                                        answer={shown()}
                                    />
                                )}
                            </Show>
                        }
                    >
                        <CommentBox
                            isInner
                            isFocused
                            placeholder="Edit the message"
                            words={message().text}
                            onCancel={() => void setEditing(false)}
                            onSubmit={(written) => {
                                chat.change(message().id, () => ({
                                    text: written,
                                    isEdited: true,
                                }));
                                setEditing(false);
                            }}
                        />
                    </Show>
                    <Show when={message().isEdited && !isEditing()}>
                        <span {...style.attrs(text.xs, styles.muted)}>edited</span>
                    </Show>
                    <ReactionRow chat={chat} message={message()} />
                    <Show when={(properties.replies?.length ?? 0) > 0}>
                        <ThreadButton
                            replies={properties.replies ?? []}
                            onOpen={properties.onThread}
                        />
                    </Show>
                    <Show
                        when={
                            properties.isAssistant && answer() !== undefined && !isBusy(message())
                        }
                    >
                        <MessageFooter>
                            <CopyButton value={words()} label="Copy" size="icon-sm" />
                            <ActionButton
                                action={retryAnswer}
                                icon={arrowsClockwise}
                                size="icon-sm"
                                variant="ghost"
                            />
                            <AnswerPicker chat={chat} message={message()} />
                        </MessageFooter>
                    </Show>
                </MessageContent>
                <Show when={!properties.isAssistant || isOwn()}>
                    <MessageToolbar aria-label="Actions">
                        <Show when={!properties.isAssistant}>
                            <ActionButton
                                action={reactToMessage}
                                icon={smiley}
                                size="icon-sm"
                                variant="ghost"
                            />
                            <Show when={properties.onThread !== undefined}>
                                <ActionButton
                                    action={replyInThread}
                                    icon={arrowBendUpLeft}
                                    size="icon-sm"
                                    variant="ghost"
                                />
                            </Show>
                        </Show>
                        <DropdownMenu>
                            <DropdownMenuTrigger
                                render={(attributes) => (
                                    <ToolbarButton
                                        {...attributes}
                                        size="icon-sm"
                                        variant="ghost"
                                        aria-label="More"
                                    >
                                        <Icon icon={dotsThree} />
                                    </ToolbarButton>
                                )}
                            />
                            <DropdownMenuContent align="end">
                                <MenuActionList />
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </MessageToolbar>
                </Show>
            </Message>
        </MessageScrollerItem>
    );
}

/** Render the words of a message. */
function Text(properties: {
    /** The Markdown. */
    readonly text: string;
}): JSX.Element {
    const blocks = useMarkdown(() => properties.text);

    return <RichText blocks={blocks()} />;
}

/** Render what a message answers, which leads there. */
function QuoteOf(properties: {
    /** The message, undefined once it is deleted. */
    readonly message: ChatMessage | undefined;
    /** The words of it answered, undefined for all of them. */
    readonly excerpt: string | undefined;
    /** Go to it. */
    readonly onGo: () => void;
}): JSX.Element {
    return (
        <Quote
            xstyle={styles.quote}
            render={(attributes) => (
                <button type="button" {...attributes} onClick={() => properties.onGo()} />
            )}
        >
            <QuoteIcon />
            <QuoteContent>
                {properties.excerpt ??
                    properties.message?.answers[properties.message.shown]?.text ??
                    properties.message?.text ??
                    "A deleted message"}
            </QuoteContent>
        </Quote>
    );
}

/** Render the reactions to a message. */
function ReactionRow(properties: {
    /** The chat. */
    readonly chat: Chat;
    /** The message. */
    readonly message: ChatMessage;
}): JSX.Element {
    const reactions = () => Object.entries(properties.message.reactions);

    return (
        <Show when={reactions().length > 0}>
            <ReactionGroup>
                <For each={reactions()} keyed={([emoji]) => emoji}>
                    {(entry) => (
                        <Reaction
                            emoji={entry()[0]}
                            count={entry()[1].length}
                            people={entry()[1].map(nameOf)}
                            pressed={entry()[1].includes(ME)}
                            onPressedChange={() =>
                                properties.chat.react(properties.message.id, entry()[0])
                            }
                        />
                    )}
                </For>
            </ReactionGroup>
        </Show>
    );
}

/** Render who answered in a thread, and when last. */
function ThreadButton(properties: {
    /** The messages in it. */
    readonly replies: readonly ChatMessage[];
    /** Open it. */
    readonly onOpen: (() => void) | undefined;
}): JSX.Element {
    // show the first few who answered
    const locale = useLocale();
    const people = () => [...new Set(properties.replies.map(({ author }) => author))].slice(0, 3);
    const count = () => properties.replies.length;

    return (
        <div>
            <Button size="sm" variant="ghost" onClick={() => properties.onOpen?.()}>
                <For each={people()}>{(author) => <MemberFace author={author} />}</For>
                {locale.number(count())} {count() === 1 ? "reply" : "replies"}
                <Show when={properties.replies.at(-1)}>
                    {(last) => <RelativeTime value={last().at} xstyle={styles.muted} />}
                </Show>
            </Button>
        </div>
    );
}

/** Report whether an answer of a message is under way. */
export function isBusy({ answers }: ChatMessage): boolean {
    return answers.some(
        ({ status, steps }) =>
            status === "writing" ||
            (status === "working" && !steps.some((step) => step.status === "waiting")),
    );
}

/** Read the question a message answers: what was said before it. */
function questionOf(chat: Chat, conversation: string, message: ChatMessage): string {
    const all = chat.find(conversation)?.messages ?? [];
    const at = all.findIndex(({ id }) => id === message.id);

    return all.slice(0, Math.max(0, at)).findLast(({ author }) => author !== ASSISTANT)?.text ?? "";
}
