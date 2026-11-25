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
    ComposerInput,
    ComposerSubmit,
} from "../composer/index.ts";
import { PICTURES } from "../example/picture.ts";
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
import { App, Face, PEOPLE } from "./app.tsx";

/** The person who wrote to the inbox. */
const MARTA = { name: "Marta Silva", initials: "MS" };

/** The milliseconds of a minute. */
const MINUTE = 60_000;

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
