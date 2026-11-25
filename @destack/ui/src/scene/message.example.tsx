import arrowBendUpLeftIcon from "@destack/icon/phosphor/arrow-bend-up-left";
import checkCircleIcon from "@destack/icon/phosphor/check-circle";
import dotsThreeIcon from "@destack/icon/phosphor/dots-three";
import plusIcon from "@destack/icon/phosphor/plus";
import tagIcon from "@destack/icon/phosphor/tag";
import userPlusIcon from "@destack/icon/phosphor/user-plus";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space } from "@destack/theme/tokens.stylex";
import { createSignal, For, type JSX, Show, useEnvironment, useLocale } from "@destack/view";
import { Activity, ActivityBadge, ActivityBody, ActivityItem } from "../activity/index.ts";
import { Badge } from "../badge/index.ts";
import { Bubble, BubbleContent, BubbleGroup, BubbleOverlay } from "../bubble/index.ts";
import { Button } from "../button/index.ts";
import { Card, CardContent, CardHeader, CardTitle } from "../card/index.ts";
import {
    Composer,
    ComposerAddon,
    ComposerFooter,
    ComposerFrame,
    ComposerHeader,
    ComposerInput,
    ComposerSubmit,
} from "../composer/index.ts";
import { Quote, QuoteContent, QuoteDismiss, QuoteIcon } from "../quote/index.ts";
import { PICTURES } from "../example/picture.ts";
import { FileItem, FileItemGroup } from "../file-item/index.ts";
import { EmojiButton } from "../example/emoji.tsx";
import { Icon } from "@destack/ui/icon";
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemTitle,
} from "../item/index.ts";
import { Marker, MarkerContent } from "../marker/index.ts";
import {
    Message,
    MessageToolbar,
    MessageAuthor,
    MessageAvatar,
    MessageContent,
    MessageFooter,
    MessageHeader,
} from "../message/index.ts";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerItem,
    MessageScrollerViewport,
} from "../message-scroller/index.ts";
import { Reaction, ReactionGroup } from "../reaction/index.ts";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "../resizable/index.ts";
import { RelativeTime } from "../relative-time/index.ts";
import { Spinner } from "../spinner/index.ts";
import { ToolbarButton } from "../toolbar/index.ts";
import { App, Face, PEOPLE } from "./app.example.tsx";

/** The person who wrote to the inbox. */
const MARTA = { name: "Marta Silva", initials: "MS" };

/** The milliseconds of a minute. */
const MINUTE = 60_000;

/** A person of the examples. */
type Person = (typeof PEOPLE)[keyof typeof PEOPLE];

/** A message of the channel. */
interface Post {
    /** The message's id. */
    readonly id: string;
    /** Who wrote it. */
    readonly person: Person;
    /** How many minutes ago it was written. */
    readonly ago: number;
    /** What it says. */
    readonly text: string;
    /** How many reacted, by their emoji. */
    readonly reactions?: Readonly<Record<string, number>>;
    /** The file it brings. */
    readonly file?: { readonly name: string; readonly size: number; readonly type: string };
}

/** The messages of the channel. */
const POSTS: readonly Post[] = [
    {
        id: "p1",
        person: PEOPLE.ada,
        ago: 1500,
        text: "The pricing page is up on staging. Two plans, one page, no asterisks.",
    },
    {
        id: "p2",
        person: PEOPLE.grace,
        ago: 1496,
        text: "Reads well. The yearly price wraps on small phones.",
    },
    {
        id: "p3",
        person: PEOPLE.grace,
        ago: 1495,
        text: "Never mind, that was my zoom.",
        reactions: { "😅": 3 },
    },
    {
        id: "p4",
        person: PEOPLE.jonas,
        ago: 95,
        text: "Demo is down to 1:58. Here is the script we recorded from.",
        file: { name: "demo-script.pdf", size: 48_120, type: "application/pdf" },
    },
    {
        id: "p5",
        person: PEOPLE.ada,
        ago: 60,
        text: "Watched it twice. Ship it.",
        reactions: { "🚀": 4, "🎉": 2 },
    },
    {
        id: "p6",
        person: PEOPLE.grace,
        ago: 12,
        text: "Press note goes out at nine. I put the embargo line on top so nobody can miss it, and the three journalists who asked get it an hour early.",
    },
    { id: "p7", person: PEOPLE.grace, ago: 11, text: "Can someone read it once more before that?" },
];

/** The styles of the scenes. */
const styles = style.create({
    conversation: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        minHeight: 0,
        minWidth: 0,
        margin: `calc(-1 * ${space[5]})`,
    },
    rows: {
        gap: space[1],
    },
    first: {
        marginBlockStart: space[3],
    },
    composer: {
        flexShrink: 0,
        paddingInline: space[4],
        paddingBlockEnd: space[4],
    },
    direct: {
        maxWidth: "40rem",
        width: "100%",
        marginInline: "auto",
    },
    inbox: {
        flexGrow: 1,
        width: "auto",
        height: "auto",
        minHeight: 0,
        margin: `calc(-1 * ${space[5]})`,
    },
    list: {
        display: { default: "block", "@media (max-width: 60rem)": "none" },
        overflowY: "auto",
        padding: space[2],
    },
    divider: {
        display: { default: null, "@media (max-width: 60rem)": "none" },
    },
    reading: {
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        minWidth: 0,
    },
    page: {
        maxWidth: "44rem",
        width: "100%",
        marginInline: "auto",
    },
    picture: {
        paddingBlock: 0,
        paddingInline: 0,
        overflow: "hidden",
    },
    image: {
        display: "block",
        width: "16rem",
        maxWidth: "100%",
        height: "auto",
    },
    files: {
        maxWidth: "20rem",
    },
    selected: {
        backgroundColor: color.muted,
    },
});

/** Render the actions of a message, over its corner. */
function PostActions(): JSX.Element {
    return (
        <MessageToolbar aria-label="Actions">
            <EmojiButton
                label="Add reaction"
                onSelect={() => {}}
                render={(button) => <ToolbarButton {...button} />}
            />
            <ToolbarButton variant="ghost" size="icon-sm" aria-label="Reply">
                <Icon icon={arrowBendUpLeftIcon} />
            </ToolbarButton>
            <ToolbarButton variant="ghost" size="icon-sm" aria-label="More">
                <Icon icon={dotsThreeIcon} />
            </ToolbarButton>
        </MessageToolbar>
    );
}

/** Render a message of the channel, without the face where the one before is the same person's. */
function PostRow(properties: {
    readonly post: Post;
    readonly isFirst: boolean;
    readonly onReply: () => void;
}): JSX.Element {
    const { clock } = useEnvironment();
    const time = clock.now() - properties.post.ago * MINUTE;

    return (
        <MessageScrollerItem
            messageId={properties.post.id}
            xstyle={properties.isFirst && styles.first}
        >
            <Message onSwipe={properties.onReply}>
                <MessageAvatar position="top">
                    <Show when={properties.isFirst}>
                        <Face person={properties.post.person} />
                    </Show>
                </MessageAvatar>
                <MessageContent>
                    <Show when={properties.isFirst}>
                        <MessageHeader>
                            <MessageAuthor>{properties.post.person.name}</MessageAuthor>
                            <RelativeTime value={time} />
                        </MessageHeader>
                    </Show>
                    <div {...style.attrs(text.sm)}>{properties.post.text}</div>
                    <Show when={properties.post.file}>
                        {(file) => (
                            <FileItemGroup aria-label="Files" xstyle={styles.files}>
                                <FileItem size="sm" file={file()} />
                            </FileItemGroup>
                        )}
                    </Show>
                    <Show when={properties.post.reactions}>
                        {(reactions) => (
                            <ReactionGroup>
                                <For each={Object.entries(reactions())}>
                                    {([emoji, count]) => <Reaction emoji={emoji} count={count} />}
                                </For>
                                <EmojiButton label="Add reaction" onSelect={() => {}} />
                            </ReactionGroup>
                        )}
                    </Show>
                </MessageContent>
                <PostActions />
            </Message>
        </MessageScrollerItem>
    );
}

/** A team's channel, whose messages one answers, reacts to and follows. */
export const sceneChannel = defineExample({
    of: MessageScroller,
    name: "channel",
    description: "a team's channel: days, people, files, reactions, replies and someone typing",
    layout: "fullscreen",
    render: () => {
        // answer a message, and add what is written at the end
        const [posts, setPosts] = createSignal(POSTS);
        const [words, setWords] = createSignal("");
        const [quoted, setQuoted] = createSignal<Post | undefined>(undefined);
        const isFirst = (index: number): boolean =>
            posts()[index]?.person !== posts()[index - 1]?.person || posts()[index]?.id === "p4";

        return (
            <App
                name="Studio"
                groups={[
                    {
                        label: "Channels",
                        places: [
                            { name: "general", icon: <Icon name="hash" /> },
                            { name: "launch", icon: <Icon name="hash" /> },
                            { name: "press", icon: <Icon name="hash" />, count: 2 },
                        ],
                    },
                ]}
                current="launch"
            >
                <div {...style.attrs(styles.conversation)}>
                    <MessageScroller autoScroll>
                        <MessageScrollerViewport>
                            <MessageScrollerContent xstyle={styles.rows}>
                                <MessageScrollerItem>
                                    <Marker variant="separator">
                                        <MarkerContent>Yesterday</MarkerContent>
                                    </Marker>
                                </MessageScrollerItem>
                                <For each={posts()}>
                                    {(post, index) => (
                                        <>
                                            <Show when={post.id === "p4"}>
                                                <MessageScrollerItem xstyle={styles.first}>
                                                    <Marker variant="separator">
                                                        <MarkerContent>Today</MarkerContent>
                                                    </Marker>
                                                </MessageScrollerItem>
                                            </Show>
                                            <PostRow
                                                post={post}
                                                isFirst={isFirst(index())}
                                                onReply={() => setQuoted(post)}
                                            />
                                        </>
                                    )}
                                </For>
                                <MessageScrollerItem xstyle={styles.first}>
                                    <Marker role="status">
                                        <MarkerContent>Jonas is typing</MarkerContent>
                                    </Marker>
                                </MessageScrollerItem>
                            </MessageScrollerContent>
                        </MessageScrollerViewport>
                        <MessageScrollerButton />
                    </MessageScroller>
                    <div {...style.attrs(styles.composer)}>
                        <Composer
                            onSubmit={() => {
                                setPosts((all) => [
                                    ...all,
                                    {
                                        id: String(all.length),
                                        person: PEOPLE.ada,
                                        ago: 0,
                                        text: words(),
                                    },
                                ]);
                                setWords("");
                                setQuoted(undefined);
                            }}
                        >
                            <ComposerFrame>
                                <ComposerHeader>
                                    <Show when={quoted()}>
                                        {(post) => (
                                            <Quote>
                                                <QuoteIcon />
                                                <QuoteContent>
                                                    <strong>{post().person.name}</strong>
                                                    <div>{post().text}</div>
                                                </QuoteContent>
                                                <QuoteDismiss
                                                    onClick={() => setQuoted(undefined)}
                                                />
                                            </Quote>
                                        )}
                                    </Show>
                                </ComposerHeader>
                                <ComposerInput
                                    placeholder="Message launch"
                                    value={words()}
                                    onInput={(event) => setWords(event.currentTarget.value)}
                                />
                                <ComposerFooter>
                                    <Button
                                        variant="ghost"
                                        size="icon"
                                        shape="round"
                                        aria-label="Attach a file"
                                    >
                                        <Icon icon={plusIcon} />
                                    </Button>
                                    <EmojiButton
                                        label="Add emoji"
                                        onSelect={(emoji) => setWords((all) => all + emoji)}
                                    />
                                    <ComposerSubmit disabled={words().trim() === ""} />
                                </ComposerFooter>
                            </ComposerFrame>
                        </Composer>
                    </div>
                </div>
            </App>
        );
    },
});

/** A conversation between two people in bubbles, with one line to write in. */
export const sceneDirect = defineExample({
    of: MessageScroller,
    name: "direct",
    description:
        "a conversation between two people in bubbles, with a picture, a reaction and one line",
    layout: "fullscreen",
    render: () => {
        // add what is sent at the end
        const [sent, setSent] = createSignal<readonly string[]>([]);
        const [words, setWords] = createSignal("");
        const locale = useLocale();

        return (
            <App
                name="Messages"
                groups={[
                    {
                        label: "People",
                        places: [
                            { name: "Grace Hopper", icon: <Icon name="chat-circle" /> },
                            { name: "Jonas Abiola", icon: <Icon name="chat-circle" />, count: 1 },
                        ],
                    },
                ]}
                current="Grace Hopper"
            >
                <div {...style.attrs(styles.conversation)}>
                    <MessageScroller autoScroll>
                        <MessageScrollerViewport>
                            <MessageScrollerContent xstyle={styles.direct}>
                                <MessageScrollerItem>
                                    <Marker variant="separator">
                                        <MarkerContent>
                                            {locale.date(1_793_000_000_000, {
                                                weekday: "long",
                                                hour: "numeric",
                                                minute: "numeric",
                                                timeZone: "UTC",
                                            })}
                                        </MarkerContent>
                                    </Marker>
                                </MessageScrollerItem>
                                <MessageScrollerItem messageId="d1">
                                    <Message>
                                        <MessageAvatar>
                                            <Face person={PEOPLE.grace} />
                                        </MessageAvatar>
                                        <MessageContent>
                                            <BubbleGroup>
                                                <Bubble variant="secondary">
                                                    <BubbleContent>
                                                        We made it up the hill before the rain
                                                    </BubbleContent>
                                                </Bubble>
                                                <Bubble variant="secondary">
                                                    <BubbleContent xstyle={styles.picture}>
                                                        <img
                                                            src={PICTURES.alfama}
                                                            alt="Roofs of Alfama above the river"
                                                            {...style.attrs(styles.image)}
                                                        />
                                                    </BubbleContent>
                                                    <BubbleOverlay>😍 1</BubbleOverlay>
                                                </Bubble>
                                            </BubbleGroup>
                                        </MessageContent>
                                    </Message>
                                </MessageScrollerItem>
                                <MessageScrollerItem messageId="d2">
                                    <Message align="end">
                                        <MessageContent>
                                            <BubbleGroup>
                                                <Bubble>
                                                    <BubbleContent>Oh that light</BubbleContent>
                                                </Bubble>
                                                <Bubble>
                                                    <BubbleContent>
                                                        Is the place with the custard tarts still
                                                        open on Sundays?
                                                    </BubbleContent>
                                                </Bubble>
                                            </BubbleGroup>
                                            <MessageFooter>Read 18:42</MessageFooter>
                                        </MessageContent>
                                    </Message>
                                </MessageScrollerItem>
                                <MessageScrollerItem messageId="d3">
                                    <Message>
                                        <MessageAvatar>
                                            <Face person={PEOPLE.grace} />
                                        </MessageAvatar>
                                        <MessageContent>
                                            <Bubble variant="secondary">
                                                <BubbleContent>
                                                    Until six! I’ll keep you one
                                                </BubbleContent>
                                            </Bubble>
                                        </MessageContent>
                                    </Message>
                                </MessageScrollerItem>
                                <For each={sent()}>
                                    {(line, index) => (
                                        <MessageScrollerItem messageId={`sent-${String(index())}`}>
                                            <Message align="end">
                                                <MessageContent>
                                                    <Bubble>
                                                        <BubbleContent>{line}</BubbleContent>
                                                    </Bubble>
                                                    <MessageFooter>
                                                        <Spinner /> Sending
                                                    </MessageFooter>
                                                </MessageContent>
                                            </Message>
                                        </MessageScrollerItem>
                                    )}
                                </For>
                            </MessageScrollerContent>
                        </MessageScrollerViewport>
                        <MessageScrollerButton />
                    </MessageScroller>
                    <div {...style.attrs(styles.composer)}>
                        <Composer
                            xstyle={styles.direct}
                            onSubmit={() => {
                                setSent((all) => [...all, words()]);
                                setWords("");
                            }}
                        >
                            <ComposerFrame>
                                <ComposerInput
                                    placeholder="Message Grace"
                                    value={words()}
                                    onInput={(event) => setWords(event.currentTarget.value)}
                                />
                                <ComposerAddon align="inline-end">
                                    <ComposerSubmit disabled={words().trim() === ""} />
                                </ComposerAddon>
                            </ComposerFrame>
                        </Composer>
                    </div>
                </div>
            </App>
        );
    },
});

/** The conversations of the inbox. */
const INBOX = [
    ["Marta Silva", "Order 2041 arrived without the lid", 8, "open"],
    ["Tomás Reis", "Can I change the delivery address?", 34, "open"],
    ["Inês Duarte", "Thank you, it works now", 190, "done"],
] as const;

/** An inbox whose conversation mixes mail, notes of the team and what happened. */
export const sceneInbox = defineExample({
    of: MessageScroller,
    name: "inbox",
    description: "an inbox: conversations beside one with mail, a note of the team and its changes",
    layout: "fullscreen",
    render: () => {
        const { clock } = useEnvironment();
        const ago = (minutes: number): number => clock.now() - minutes * MINUTE;

        return (
            <App
                name="Support"
                groups={[
                    {
                        label: "Inboxes",
                        places: [
                            { name: "Open", icon: <Icon name="tray" />, count: 2 },
                            { name: "Mine", icon: <Icon name="user" /> },
                            { name: "Done", icon: <Icon name="check-circle" /> },
                        ],
                    },
                ]}
                current="Open"
                actions={
                    <Button variant="outline">
                        <Icon icon={checkCircleIcon} />
                        Close
                    </Button>
                }
            >
                <ResizablePanelGroup xstyle={styles.inbox}>
                    <ResizablePanel
                        defaultSize={32}
                        minSize="220px"
                        maxSize={50}
                        xstyle={styles.list}
                    >
                        <ItemGroup>
                            <For each={INBOX}>
                                {([person, words, minutes, status], index) => (
                                    <Item size="sm" xstyle={index() === 0 && styles.selected}>
                                        <ItemContent>
                                            <ItemTitle>{person}</ItemTitle>
                                            <ItemDescription>{words}</ItemDescription>
                                        </ItemContent>
                                        <ItemActions xstyle={text.xs}>
                                            <Show when={status === "done"}>
                                                <Badge color="green">Done</Badge>
                                            </Show>
                                            <RelativeTime value={ago(minutes)} length="narrow" />
                                        </ItemActions>
                                    </Item>
                                )}
                            </For>
                        </ItemGroup>
                    </ResizablePanel>
                    <ResizableHandle aria-label="Resize the list" xstyle={styles.divider} />
                    <ResizablePanel minSize="320px" xstyle={styles.reading}>
                        <MessageScroller>
                            <MessageScrollerViewport>
                                <MessageScrollerContent xstyle={styles.page}>
                                    <MessageScrollerItem messageId="m1" scrollAnchor>
                                        <Message>
                                            <MessageAvatar position="top">
                                                <Face person={MARTA} />
                                            </MessageAvatar>
                                            <MessageContent>
                                                <MessageHeader>
                                                    <MessageAuthor>Marta Silva</MessageAuthor>
                                                    to support
                                                    <RelativeTime value={ago(52)} />
                                                </MessageHeader>
                                                <Card size="sm">
                                                    <CardHeader>
                                                        <CardTitle>
                                                            Order 2041 arrived without the lid
                                                        </CardTitle>
                                                    </CardHeader>
                                                    <CardContent>
                                                        Hello, the teapot came today and it is
                                                        lovely, but the lid is missing from the box.
                                                        Could you send one?
                                                    </CardContent>
                                                </Card>
                                            </MessageContent>
                                        </Message>
                                    </MessageScrollerItem>
                                    <MessageScrollerItem>
                                        <Marker variant="separator">
                                            <MarkerContent>
                                                Ada took this conversation
                                            </MarkerContent>
                                        </Marker>
                                    </MessageScrollerItem>
                                    <MessageScrollerItem messageId="m2">
                                        <Message>
                                            <MessageAvatar position="top">
                                                <Face person={PEOPLE.jonas} />
                                            </MessageAvatar>
                                            <MessageContent>
                                                <MessageHeader>
                                                    <MessageAuthor>Jonas Abiola</MessageAuthor>
                                                    <Badge color="yellow">Note</Badge>
                                                    <RelativeTime value={ago(31)} />
                                                </MessageHeader>
                                                <Bubble variant="tinted">
                                                    <BubbleContent>
                                                        Third one from that batch. I asked the
                                                        warehouse to open the rest.
                                                    </BubbleContent>
                                                </Bubble>
                                            </MessageContent>
                                        </Message>
                                    </MessageScrollerItem>
                                    <MessageScrollerItem messageId="m3">
                                        <Message align="end">
                                            <MessageAvatar position="top">
                                                <Face person={PEOPLE.ada} />
                                            </MessageAvatar>
                                            <MessageContent>
                                                <Bubble variant="secondary">
                                                    <BubbleContent>
                                                        Hello Marta, I’m sorry about that. A lid
                                                        leaves our warehouse today and should be
                                                        with you on Thursday.
                                                    </BubbleContent>
                                                </Bubble>
                                                <MessageFooter>
                                                    Sent <RelativeTime value={ago(8)} />
                                                </MessageFooter>
                                            </MessageContent>
                                        </Message>
                                    </MessageScrollerItem>
                                </MessageScrollerContent>
                            </MessageScrollerViewport>
                            <MessageScrollerButton />
                        </MessageScroller>
                        <div {...style.attrs(styles.composer)}>
                            <Composer
                                submitMode="ctrlEnter"
                                xstyle={styles.page}
                                onSubmit={() => {}}
                            >
                                <ComposerFrame>
                                    <ComposerInput placeholder="Reply to Marta" rows={2} />
                                    <ComposerFooter>
                                        <Button variant="ghost" size="sm">
                                            Saved replies
                                        </Button>
                                        <Button variant="ghost" size="sm">
                                            Note
                                        </Button>
                                        <ComposerSubmit size="sm">Send and close</ComposerSubmit>
                                    </ComposerFooter>
                                </ComposerFrame>
                            </Composer>
                        </div>
                    </ResizablePanel>
                </ResizablePanelGroup>
            </App>
        );
    },
});

/** What happened to a task, with comments among the changes. */
export const sceneActivity = defineExample({
    of: Activity,
    name: "activity",
    description: "a task's activity: its changes, comments with reactions, and a box to comment in",
    layout: "padded",
    render: () => {
        const { clock } = useEnvironment();
        const ago = (minutes: number): number => clock.now() - minutes * MINUTE;

        return (
            <div {...style.attrs(styles.page)}>
                <Activity aria-label="Activity">
                    <ActivityItem condensed>
                        <ActivityBadge>
                            <Face person={PEOPLE.ada} />
                        </ActivityBadge>
                        <ActivityBody>
                            <span>
                                Ada created the task · <RelativeTime value={ago(4400)} />
                            </span>
                        </ActivityBody>
                    </ActivityItem>
                    <ActivityItem condensed>
                        <ActivityBadge>
                            <Icon icon={userPlusIcon} />
                        </ActivityBadge>
                        <ActivityBody>
                            <span>
                                Ada assigned Grace · <RelativeTime value={ago(4390)} />
                            </span>
                        </ActivityBody>
                    </ActivityItem>
                    <ActivityItem condensed>
                        <ActivityBadge>
                            <Icon icon={tagIcon} />
                        </ActivityBadge>
                        <ActivityBody>
                            <span>
                                Grace added <Badge color="orange">Press</Badge> and{" "}
                                <Badge color="blue">Launch</Badge> ·{" "}
                                <RelativeTime value={ago(1500)} />
                            </span>
                        </ActivityBody>
                    </ActivityItem>
                    <ActivityItem>
                        <ActivityBadge>
                            <Face person={PEOPLE.grace} />
                        </ActivityBadge>
                        <ActivityBody>
                            <Card>
                                <CardContent>
                                    <Message>
                                        <MessageContent>
                                            <MessageHeader>
                                                <MessageAuthor>Grace Hopper</MessageAuthor>
                                                <RelativeTime value={ago(190)} />
                                            </MessageHeader>
                                            <div>
                                                First draft is in the page. The quote still needs a
                                                name under it.
                                            </div>
                                            <ReactionGroup>
                                                <Reaction emoji="👀" count={2} pressed />
                                                <EmojiButton
                                                    label="Add reaction"
                                                    onSelect={() => {}}
                                                />
                                            </ReactionGroup>
                                        </MessageContent>
                                        <PostActions />
                                    </Message>
                                </CardContent>
                            </Card>
                        </ActivityBody>
                    </ActivityItem>
                    <ActivityItem condensed>
                        <ActivityBadge color="green">
                            <Icon icon={checkCircleIcon} />
                        </ActivityBadge>
                        <ActivityBody>
                            <span>
                                Ada moved it to <strong>In review</strong> ·{" "}
                                <RelativeTime value={ago(20)} />
                            </span>
                        </ActivityBody>
                    </ActivityItem>
                </Activity>
                <Composer submitMode="ctrlEnter" xstyle={styles.first} onSubmit={() => {}}>
                    <ComposerFrame>
                        <ComposerInput placeholder="Leave a comment" />
                        <ComposerFooter>
                            <Button
                                variant="ghost"
                                size="icon"
                                shape="round"
                                aria-label="Attach a file"
                            >
                                <Icon icon={plusIcon} />
                            </Button>
                            <ComposerSubmit size="sm">Comment</ComposerSubmit>
                        </ComposerFooter>
                    </ComposerFrame>
                </Composer>
            </div>
        );
    },
});
