import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Stack } from "../stack/index.ts";
import { Bubble, BubbleContent, BubbleGroup, BubbleOverlay } from "./bubble.tsx";

/** A conversation between two people, each person's bubbles in a row. */
export const bubbleConversation = defineExample({
    of: Bubble,
    name: "conversation",
    description: t`A conversation between two people, each person's bubbles tight where they meet.`,
    layout: "padded",
    properties: {
        variant: "soft",
        color: "gray",
    },
    render: (properties) => (
        <Stack space="4">
            <BubbleGroup>
                <Bubble {...properties}>
                    <BubbleContent>Hey, are we still on for Saturday?</BubbleContent>
                </Bubble>
                <Bubble {...properties}>
                    <BubbleContent>
                        I found a place by the river that takes the four of us, and they have a
                        table outside if the weather holds
                    </BubbleContent>
                </Bubble>
            </BubbleGroup>
            <BubbleGroup>
                <Bubble align="end">
                    <BubbleContent>Yes!</BubbleContent>
                </Bubble>
                <Bubble align="end">
                    <BubbleContent>Outside sounds lovely</BubbleContent>
                </Bubble>
                <Bubble align="end">
                    <BubbleContent>I’ll bring the good camera</BubbleContent>
                </Bubble>
            </BubbleGroup>
        </Stack>
    ),
});

/** A booked ferry, confirmed in a soft bubble. */
export const bubbleBooking = defineExample({
    of: Bubble,
    name: "booking",
    description: t`A booked ferry, confirmed in a soft bubble.`,
    layout: "padded",
    properties: {
        variant: "soft",
    },
    render: (properties) => (
        <Bubble {...properties}>
            <BubbleContent>
                Booked: two seats by the window on the nine o'clock ferry.
            </BubbleContent>
        </Bubble>
    ),
});

/** Two reactions over the edge of a bubble. */
export const bubbleOverlayReactions = defineExample({
    of: BubbleOverlay,
    name: "reactions",
    description: t`Two reactions over the edge of a bubble.`,
    layout: "padded",
    properties: {
        role: "img",
        "aria-label": "Reactions: heart, thumbs up",
    },
    render: (properties) => (
        <Bubble variant="soft" color="gray">
            <BubbleContent>
                I found a place by the river that takes the four of us, and they have a table
                outside if the weather holds
            </BubbleContent>
            <BubbleOverlay {...properties}>
                <span>❤️</span>
                <span>👍</span>
                <span>2</span>
            </BubbleOverlay>
        </Bubble>
    ),
});
