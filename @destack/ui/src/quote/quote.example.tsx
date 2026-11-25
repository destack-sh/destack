import arrowElbowUpLeftIcon from "@destack/icon/phosphor/arrow-elbow-up-left";
import { defineExample } from "@destack/package/declare";
import { createSignal, Show } from "@destack/view";
import { Bubble, BubbleContent } from "../bubble/index.ts";
import {
    Composer,
    ComposerFooter,
    ComposerFrame,
    ComposerHeader,
    ComposerInput,
    ComposerSubmit,
} from "../composer/index.ts";
import { Icon } from "../icon/index.ts";
import { Message, MessageContent } from "../message/index.ts";
import { Stack } from "../stack/index.ts";
import { Quote, QuoteContent, QuoteDismiss, QuoteIcon } from "./quote.tsx";

/** What a sent message answers, over its words, which leads there. */
export const quoteMessage = defineExample({
    of: Quote,
    name: "message",
    description: "what sent messages answer, over their words, which leads there",
    layout: "padded",
    render: () => (
        <Stack space="4">
            <Message align="start">
                <MessageContent>
                    <Bubble variant="muted">
                        <BubbleContent>
                            <Stack space="1">
                                <Quote render={(attributes) => <a {...attributes} href="#when" />}>
                                    <QuoteContent>
                                        When does the client need the quotes?
                                    </QuoteContent>
                                </Quote>
                                <span>By noon, please.</span>
                            </Stack>
                        </BubbleContent>
                    </Bubble>
                </MessageContent>
            </Message>
            <Message align="end">
                <MessageContent>
                    <Bubble>
                        <BubbleContent>
                            <Stack space="1">
                                <Quote render={(attributes) => <a {...attributes} href="#noon" />}>
                                    <QuoteIcon />
                                    <QuoteContent>By noon, please.</QuoteContent>
                                </Quote>
                                <span>They are in the notebook.</span>
                            </Stack>
                        </BubbleContent>
                    </Bubble>
                </MessageContent>
            </Message>
        </Stack>
    ),
});

/** What a message being written answers, which can be taken back. */
export const quoteComposer = defineExample({
    of: Quote,
    name: "composer",
    description: "what a message being written answers, which can be taken back",
    layout: "padded",
    render: () => {
        const [isQuoted, setQuoted] = createSignal(true);

        return (
            <Composer>
                <ComposerFrame>
                    <ComposerHeader>
                        <Show when={isQuoted()}>
                            <Quote>
                                <QuoteIcon>
                                    <Icon icon={arrowElbowUpLeftIcon} />
                                </QuoteIcon>
                                <QuoteContent>By noon, please.</QuoteContent>
                                <QuoteDismiss onClick={() => setQuoted(false)} />
                            </Quote>
                        </Show>
                    </ComposerHeader>
                    <ComposerInput placeholder="Reply" />
                    <ComposerFooter>
                        <ComposerSubmit />
                    </ComposerFooter>
                </ComposerFrame>
            </Composer>
        );
    },
});
