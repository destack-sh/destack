import type { JSX } from "@solidjs/web";
import { Avatar, AvatarFallback } from "../avatar/index.ts";
import { Bubble, BubbleContent } from "../bubble/index.ts";
import {
    Message,
    MessageAvatar,
    MessageContent,
    MessageFooter,
    MessageGroup,
    MessageHeader,
} from "./message.tsx";

/** Show a reply under a note: Ada's question and the reader's own answer. */
export function MessageExample(): JSX.Element {
    return (
        <MessageGroup>
            <Message>
                <MessageAvatar>
                    <Avatar size="sm">
                        <AvatarFallback>AL</AvatarFallback>
                    </Avatar>
                </MessageAvatar>
                <MessageContent>
                    <MessageHeader>Ada Lovelace</MessageHeader>
                    <Bubble variant="muted">
                        <BubbleContent>Is the Lisbon trip in May or June?</BubbleContent>
                    </Bubble>
                </MessageContent>
            </Message>
            <Message align="end">
                <MessageContent>
                    <Bubble>
                        <BubbleContent>June, the second week.</BubbleContent>
                    </Bubble>
                    <MessageFooter>Read</MessageFooter>
                </MessageContent>
            </Message>
        </MessageGroup>
    );
}
