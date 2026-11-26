import { t } from "@destack/locale";
import arrowBendUpLeftIcon from "@destack/icon/phosphor/arrow-bend-up-left";
import dotsThreeIcon from "@destack/icon/phosphor/dots-three";
import smileyIcon from "@destack/icon/phosphor/smiley";
import { defineExample } from "@destack/package/declare";
import { useEnvironment } from "@destack/view";
import { Avatar, AvatarFallback, AvatarImage } from "../avatar/index.ts";
import { Bubble, BubbleContent, BubbleGroup } from "../bubble/index.ts";
import { Icon } from "../icon/index.ts";
import { PICTURES } from "../example/picture.ts";
import { Reaction, ReactionGroup } from "../reaction/index.ts";
import { RelativeTime } from "../relative-time/index.ts";
import { Stack } from "../stack/index.ts";
import { ToolbarButton } from "../toolbar/index.ts";
import {
    Message,
    MessageToolbar,
    MessageAuthor,
    MessageAvatar,
    MessageContent,
    MessageFooter,
    MessageGroup,
    MessageHeader,
} from "./message.tsx";

/** The milliseconds of a minute. */
const MINUTE = 60_000;

/** Two messages of a team's channel, with actions over the one pointed at. */
export const messageChannel = defineExample({
    of: Message,
    name: "channel",
    description: t`Two messages of a team's channel, with actions over the one pointed at.`,
    layout: "padded",
    render: (properties) => {
        const { clock } = useEnvironment();

        return (
            <Stack space="4">
                <Message {...properties}>
                    <MessageAvatar position="top">
                        <Avatar>
                            <AvatarImage src={PICTURES.ada} alt="" />
                            <AvatarFallback>AL</AvatarFallback>
                        </Avatar>
                    </MessageAvatar>
                    <MessageContent>
                        <MessageHeader>
                            <MessageAuthor>Ada Lovelace</MessageAuthor>
                            <RelativeTime value={clock.now() - 12 * MINUTE} />
                        </MessageHeader>
                        <div>
                            The engine ran the whole table overnight, without one wrong digit.
                        </div>
                        <ReactionGroup>
                            <Reaction
                                emoji="🎉"
                                count={3}
                                pressed
                                people={["Grace", "You", "Alan"]}
                            />
                            <Reaction emoji="👏" count={1} people={["Katherine"]} />
                        </ReactionGroup>
                    </MessageContent>
                    <MessageToolbar aria-label="Actions">
                        <ToolbarButton variant="ghost" isIconOnly size="sm" aria-label="React">
                            <Icon icon={smileyIcon} />
                        </ToolbarButton>
                        <ToolbarButton variant="ghost" isIconOnly size="sm" aria-label="Reply">
                            <Icon icon={arrowBendUpLeftIcon} />
                        </ToolbarButton>
                        <ToolbarButton variant="ghost" isIconOnly size="sm" aria-label="More">
                            <Icon icon={dotsThreeIcon} />
                        </ToolbarButton>
                    </MessageToolbar>
                </Message>
                <Message {...properties}>
                    <MessageAvatar position="top">
                        <Avatar>
                            <AvatarImage src={PICTURES.grace} alt="" />
                            <AvatarFallback>GH</AvatarFallback>
                        </Avatar>
                    </MessageAvatar>
                    <MessageContent>
                        <MessageHeader>
                            <MessageAuthor>Grace Hopper</MessageAuthor>
                            <RelativeTime value={clock.now() - 4 * MINUTE} />
                        </MessageHeader>
                        <div>Then let’s print it.</div>
                    </MessageContent>
                </Message>
            </Stack>
        );
    },
});

/** A conversation in bubbles, one's own at the end. */
export const messageDirect = defineExample({
    of: Message,
    name: "direct",
    description: t`A conversation in bubbles, one's own at the end, with whether it was read.`,
    layout: "padded",
    render: (properties) => (
        <Stack space="4">
            <Message {...properties}>
                <MessageAvatar>
                    <Avatar>
                        <AvatarImage src={PICTURES.grace} alt="Grace Hopper" />
                        <AvatarFallback>GH</AvatarFallback>
                    </Avatar>
                </MessageAvatar>
                <MessageContent>
                    <BubbleGroup>
                        <Bubble variant="soft" color="gray">
                            <BubbleContent>Did the moth survive?</BubbleContent>
                        </Bubble>
                        <Bubble variant="soft" color="gray">
                            <BubbleContent>Asking for the logbook.</BubbleContent>
                        </Bubble>
                    </BubbleGroup>
                </MessageContent>
            </Message>
            <MessageGroup>
                <Message align="end">
                    <MessageContent>
                        <Bubble>
                            <BubbleContent>It’s taped to page 92.</BubbleContent>
                        </Bubble>
                        <MessageFooter>Read</MessageFooter>
                    </MessageContent>
                </Message>
            </MessageGroup>
        </Stack>
    ),
});

/** Grace's picture beside a long bubble, at its top or its tail. */
export const messageAvatarBeside = defineExample({
    of: MessageAvatar,
    name: "beside",
    description: t`Grace's picture beside a long bubble, at its top or its tail.`,
    layout: "padded",
    render: (properties) => (
        <Message>
            <MessageAvatar {...properties}>
                <Avatar>
                    <AvatarImage src={PICTURES.grace} alt="Grace Hopper" />
                    <AvatarFallback>GH</AvatarFallback>
                </Avatar>
            </MessageAvatar>
            <MessageContent>
                <Bubble variant="soft" color="gray">
                    <BubbleContent>
                        The relay in panel F stuck again. I taped the moth into the logbook and ran
                        the whole table a second time to be sure.
                    </BubbleContent>
                </Bubble>
            </MessageContent>
        </Message>
    ),
});

/** Ada's message with its actions in view. */
export const messageToolbarActions = defineExample({
    of: MessageToolbar,
    name: "actions",
    description: t`Ada's message with its actions in view.`,
    layout: "padded",
    properties: {
        autohide: "never",
        "aria-label": "Actions",
    },
    render: (properties) => (
        <Message>
            <MessageContent>
                <MessageHeader>
                    <MessageAuthor>Ada Lovelace</MessageAuthor>
                </MessageHeader>
                <div>The engine ran the whole table overnight.</div>
                <MessageFooter>
                    <MessageToolbar {...properties}>
                        <ToolbarButton variant="ghost" isIconOnly size="sm" aria-label="React">
                            <Icon icon={smileyIcon} />
                        </ToolbarButton>
                        <ToolbarButton variant="ghost" isIconOnly size="sm" aria-label="Reply">
                            <Icon icon={arrowBendUpLeftIcon} />
                        </ToolbarButton>
                    </MessageToolbar>
                </MessageFooter>
            </MessageContent>
        </Message>
    ),
});
