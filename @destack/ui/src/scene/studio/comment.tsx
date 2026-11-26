import dotsThree from "@destack/icon/phosphor/dots-three";
import arrowBendUpLeft from "@destack/icon/phosphor/arrow-bend-up-left";
import smiley from "@destack/icon/phosphor/smiley";
import { Place } from "@destack/object";
import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";
import { createSignal, For, handle, type JSX, provide, Show, untrack } from "@destack/view";
import { createAutofocus } from "@destack/view/primitives/focus";
import { ActionButton, MenuActionList } from "../../action/index.ts";
import { Button } from "../../button/index.ts";
import { Card, CardContent } from "../../card/index.ts";
import {
    Composer,
    ComposerFooter,
    ComposerFrame,
    ComposerInput,
    ComposerSubmit,
} from "../../composer/index.ts";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuTrigger,
} from "../../dropdown-menu/index.ts";
import { Icon } from "../../icon/index.ts";
import {
    Message,
    MessageAuthor,
    MessageAvatar,
    MessageContent,
    MessageHeader,
    MessageToolbar,
} from "../../message/index.ts";
import { Reaction, ReactionGroup } from "../../reaction/index.ts";
import { RelativeTime } from "../../relative-time/index.ts";
import { Stack } from "../../stack/index.ts";
import { ToolbarButton } from "../../toolbar/index.ts";
import {
    answerComment,
    copyComment,
    deleteComment,
    editComment,
    reactToComment,
} from "./actions.example.ts";
import { Face } from "./shell.tsx";
import { personOf, referenceOfComment, type Studio, type TaskEvent } from "./state.ts";

/** The reactions people pick from. */
const EMOJIS = ["👍", "❤️", "🎉", "👀", "🙏"];

/** The styles of comments. */
const styles = style.create({
    words: {
        color: color.foreground,
        whiteSpace: "pre-wrap",
        overflowWrap: "anywhere",
    },
    answers: {
        paddingBlockStart: space[3],
        borderBlockStartWidth: stroke.border,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: color.border,
    },
});

/** Render the box a comment is written in. */
export function CommentBox(properties: {
    /** What it asks for. */
    readonly placeholder: string;
    /** The words it starts with. */
    readonly words?: string;
    /** Whether it takes the focus as it shows. */
    readonly isFocused?: boolean;
    /** Whether it stands in a card. */
    readonly isInner?: boolean;
    /** Take the words. */
    readonly onSubmit: (words: string) => void;
    /** Leave it, absent for a box that stays. */
    readonly onCancel?: () => void;
}): JSX.Element {
    // take the focus where it opened for it
    const [words, setWords] = createSignal(untrack(() => properties.words ?? ""));
    const [input, setInput] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    createAutofocus(() => properties.isFocused === true && input());

    return (
        <Composer
            submitMode="ctrlEnter"
            onSubmit={() => {
                properties.onSubmit(words().trim());
                setWords("");
            }}
            onKeyDown={(event) => {
                // leave it by Escape
                if (event.key === "Escape" && properties.onCancel !== undefined) {
                    event.preventDefault();
                    properties.onCancel();
                }
            }}
        >
            <ComposerFrame size="sm" variant={properties.isInner === true ? "muted" : "raised"}>
                <ComposerInput
                    aria-label={properties.placeholder}
                    placeholder={properties.placeholder}
                    ref={(element) => setInput(element)}
                    value={words()}
                    onInput={(event) => setWords(event.currentTarget.value)}
                />
                <ComposerFooter>
                    <Show when={properties.onCancel}>
                        {(cancel) => (
                            <Button size="sm" variant="ghost" onClick={() => cancel()()}>
                                Cancel
                            </Button>
                        )}
                    </Show>
                    <ComposerSubmit isIconOnly size="sm" disabled={words().trim() === ""} />
                </ComposerFooter>
            </ComposerFrame>
        </Composer>
    );
}

/** Render a comment and the answers to it, in one card. */
export function CommentCard(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** The task it is about. */
    readonly task: string;
    /** The comment. */
    readonly comment: TaskEvent;
    /** The answers to it, the oldest first. */
    readonly answers: readonly TaskEvent[];
}): JSX.Element {
    const [isAnswering, setAnswering] = createSignal(false);

    return (
        <Card size="sm" ref={handle(answerComment, { run: () => void setAnswering(true) })}>
            <CardContent>
                <Stack space="3">
                    <Comment studio={properties.studio} comment={properties.comment} />
                    <For each={properties.answers} keyed={(answer) => answer.id}>
                        {(answer) => (
                            <div {...style.attrs(styles.answers)}>
                                <Comment studio={properties.studio} comment={answer()} />
                            </div>
                        )}
                    </For>
                    <Show when={isAnswering()}>
                        <CommentBox
                            isInner
                            isFocused
                            placeholder="Leave a reply…"
                            onCancel={() => void setAnswering(false)}
                            onSubmit={(words) => {
                                properties.studio.comment(
                                    properties.task,
                                    words,
                                    properties.comment.id,
                                );
                                setAnswering(false);
                            }}
                        />
                    </Show>
                </Stack>
            </CardContent>
        </Card>
    );
}

/** Render one comment: who wrote it and when, its words and the reactions to it. */
function Comment(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** The comment. */
    readonly comment: TaskEvent;
}): JSX.Element {
    // react to it, and change it where the person wrote it
    const { studio } = properties;
    const [isEditing, setEditing] = createSignal(false);
    const id = () => properties.comment.id;
    const person = () => personOf(properties.comment.person);
    const isOwn = () => properties.comment.person === "ada";
    const reactions = () =>
        Object.entries(properties.comment.reactions ?? {}).filter(
            ([, people]) => people.length > 0,
        );

    return (
        <Message
            tabindex={0}
            ref={[
                provide(() => Place.current(referenceOfComment(id()))),
                handle(reactToComment, {
                    choices: () => EMOJIS.map((emoji) => ({ value: emoji, title: emoji })),
                    chosen: () =>
                        reactions().flatMap(([emoji, people]) =>
                            people.includes("ada") ? [emoji] : [],
                        ),
                    run: ({ values }) => studio.react(id(), values.emoji),
                }),
                handle(editComment, {
                    isEnabled: isOwn,
                    run: () => void setEditing(true),
                }),
                handle(copyComment, {
                    run: () => navigator.clipboard.writeText(properties.comment.text ?? ""),
                }),
                handle(deleteComment, {
                    isEnabled: isOwn,
                    run: () => studio.removeComment(id()),
                }),
            ]}
        >
            <MessageAvatar position="top">
                <Show when={person()}>{(each) => <Face person={each()} />}</Show>
            </MessageAvatar>
            <MessageContent>
                <MessageHeader>
                    <MessageAuthor>{person()?.name ?? "Somebody"}</MessageAuthor>
                    <RelativeTime value={properties.comment.at} />
                </MessageHeader>
                <Show
                    when={isEditing()}
                    fallback={<div {...style.attrs(styles.words)}>{properties.comment.text}</div>}
                >
                    <CommentBox
                        isInner
                        isFocused
                        placeholder="Edit the comment"
                        words={properties.comment.text ?? ""}
                        onCancel={() => void setEditing(false)}
                        onSubmit={(words) => {
                            studio.editComment(id(), words);
                            setEditing(false);
                        }}
                    />
                </Show>
                <Show when={reactions().length > 0}>
                    <ReactionGroup>
                        <For each={reactions()} keyed={([emoji]) => emoji}>
                            {(entry) => (
                                <Reaction
                                    emoji={entry()[0]}
                                    count={entry()[1].length}
                                    people={entry()[1].map((each) => personOf(each)?.name ?? "")}
                                    pressed={entry()[1].includes("ada")}
                                    onPressedChange={() => studio.react(id(), entry()[0])}
                                />
                            )}
                        </For>
                    </ReactionGroup>
                </Show>
            </MessageContent>
            <MessageToolbar aria-label="Actions">
                <ActionButton
                    action={reactToComment}
                    icon={smiley}
                    isIconOnly
                    size="sm"
                    variant="ghost"
                />
                <ActionButton
                    action={answerComment}
                    icon={arrowBendUpLeft}
                    isIconOnly
                    size="sm"
                    variant="ghost"
                />
                <DropdownMenu>
                    <DropdownMenuTrigger
                        render={(attributes) => (
                            <ToolbarButton
                                {...attributes}
                                isIconOnly
                                size="sm"
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
        </Message>
    );
}
