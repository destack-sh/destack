import { defineExample } from "@destack/package/declare";
import { For } from "@destack/view";
import { Stack } from "../stack/index.ts";
import { Bubble, BubbleContent, BubbleGroup, BubbleOverlay } from "./bubble.tsx";

/** The variants, each with a message it suits. */
const VARIANTS = [
    ["default", "The ferry leaves at nine."],
    ["secondary", "Then we have breakfast there."],
    ["muted", "Ada missed a call"],
    ["tinted", "Booked: two seats by the window."],
    ["outline", "Which deck did you take?"],
    ["ghost", "The upper deck has the better view, and the wind that comes with it."],
    ["destructive", "Not delivered"],
] as const;

/** A conversation between two people, each person's bubbles in a row. */
export const bubbleConversation = defineExample({
    of: Bubble,
    name: "conversation",
    description: "a conversation between two people, each person's bubbles tight where they meet",
    layout: "padded",
    render: () => (
        <Stack space="4">
            <BubbleGroup>
                <Bubble variant="secondary">
                    <BubbleContent>Hey, are we still on for Saturday?</BubbleContent>
                </Bubble>
                <Bubble variant="secondary">
                    <BubbleContent>
                        I found a place by the river that takes the four of us, and they have a
                        table outside if the weather holds
                    </BubbleContent>
                    <BubbleOverlay role="img" aria-label="ReactionGroup: heart, thumbs up">
                        <span>❤️</span>
                        <span>👍</span>
                        <span>2</span>
                    </BubbleOverlay>
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

/** Each variant. */
export const bubbleVariants = defineExample({
    of: Bubble,
    name: "variants",
    description: "each variant with a message it suits",
    layout: "padded",
    render: () => (
        <Stack space="3">
            <For each={VARIANTS}>
                {([variant, words]) => (
                    <Bubble variant={variant}>
                        <BubbleContent>{words}</BubbleContent>
                    </Bubble>
                )}
            </For>
        </Stack>
    ),
});
