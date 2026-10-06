import { defineExample } from "@destack/package/declare";
import { Bubble, BubbleContent, BubbleGroup, BubbleReactions } from "./bubble.tsx";

/** Two messages in a row, the second with reactions. */
export const bubbleReactions = defineExample({
    of: Bubble,
    name: "reactions",
    description: "two messages in a row, the second with reactions",
    render: () => (
        <BubbleGroup>
            <Bubble variant="secondary">
                <BubbleContent>The tickets are booked.</BubbleContent>
            </Bubble>
            <Bubble variant="secondary">
                <BubbleContent>Window seats both ways.</BubbleContent>
                <BubbleReactions aria-label="Reactions">🎉 2</BubbleReactions>
            </Bubble>
        </BubbleGroup>
    ),
});
