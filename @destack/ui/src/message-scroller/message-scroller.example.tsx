import * as style from "@destack/style";
import type { JSX } from "@solidjs/web";
import { For } from "solid-js";
import { Bubble, BubbleContent } from "../bubble/index.ts";
import { Message, MessageContent } from "../message/index.ts";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerItem,
    MessageScrollerViewport,
} from "./message-scroller.tsx";

/** The height of the conversation. */
const styles = style.create({
    conversation: { height: "24rem" },
});

/** The messages of a conversation about a trip. */
const MESSAGES = [
    { id: "1", mine: false, text: "Is the Lisbon trip in May or June?" },
    { id: "2", mine: true, text: "June, the second week." },
    { id: "3", mine: false, text: "Then I'll book the tram tour." },
];

/** Show a conversation that keeps its newest message in view and offers a jump back to it. */
export function MessageScrollerExample(): JSX.Element {
    return (
        <MessageScroller style={styles.conversation}>
            <MessageScrollerViewport aria-label="Conversation">
                <For each={MESSAGES}>
                    {(message) => (
                        <MessageScrollerItem>
                            <Message align={message.mine ? "end" : "start"}>
                                <MessageContent>
                                    <Bubble variant={message.mine ? "default" : "muted"}>
                                        <BubbleContent>{message.text}</BubbleContent>
                                    </Bubble>
                                </MessageContent>
                            </Message>
                        </MessageScrollerItem>
                    )}
                </For>
            </MessageScrollerViewport>
            <MessageScrollerButton />
        </MessageScroller>
    );
}
