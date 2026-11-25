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
    description: "two messages of a team's channel, with actions over the one pointed at",
    layout: "padded",
    render: () => {
        const { clock } = useEnvironment();

        return (
            <Stack space="4">
                <Message>
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
                        <ToolbarButton variant="ghost" size="icon-sm" aria-label="React">
                            <Icon icon={smileyIcon} />
                        </ToolbarButton>
                        <ToolbarButton variant="ghost" size="icon-sm" aria-label="Reply">
                            <Icon icon={arrowBendUpLeftIcon} />
                        </ToolbarButton>
                        <ToolbarButton variant="ghost" size="icon-sm" aria-label="More">
                            <Icon icon={dotsThreeIcon} />
                        </ToolbarButton>
                    </MessageToolbar>
                </Message>
                <Message>
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
    description: "a conversation in bubbles, one's own at the end, with whether it was read",
    layout: "padded",
    render: () => (
        <Stack space="4">
            <Message>
                <MessageAvatar>
                    <Avatar>
                        <AvatarImage src={PICTURES.grace} alt="Grace Hopper" />
                        <AvatarFallback>GH</AvatarFallback>
                    </Avatar>
                </MessageAvatar>
                <MessageContent>
                    <BubbleGroup>
                        <Bubble variant="secondary">
                            <BubbleContent>Did the moth survive?</BubbleContent>
                        </Bubble>
                        <Bubble variant="secondary">
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
