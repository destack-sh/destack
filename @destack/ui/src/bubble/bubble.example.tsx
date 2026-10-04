import type { JSX } from "@solidjs/web";
import { Bubble, BubbleContent, BubbleGroup, BubbleReactions } from "./bubble.tsx";

/** Show two messages in a row, the second with reactions. */
export function BubbleExample(): JSX.Element {
    return (
        <BubbleGroup>
            <Bubble variant="secondary">
                <BubbleContent>The tickets are booked.</BubbleContent>
            </Bubble>
            <Bubble variant="secondary">
                <BubbleContent>Window seats both ways.</BubbleContent>
                <BubbleReactions aria-label="Reactions">🎉 2</BubbleReactions>
            </Bubble>
        </BubbleGroup>
    );
}
