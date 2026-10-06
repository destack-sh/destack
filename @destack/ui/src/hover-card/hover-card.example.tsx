import { defineExample } from "@destack/package/declare";
import { HoverCard, HoverCardContent, HoverCardTrigger } from "./hover-card.tsx";

/** A person's profile while the pointer rests on a mention. */
export const hoverCardMention = defineExample({
    of: HoverCard,
    name: "mention",
    description: "a person's profile while the pointer rests on a mention",
    render: () => (
        <HoverCard>
            <HoverCardTrigger href="/people/ada">@ada</HoverCardTrigger>
            <HoverCardContent>
                <strong>Ada Lovelace</strong>
                <p>Wrote the first published program.</p>
            </HoverCardContent>
        </HoverCard>
    ),
});

/** The mention's profile card open below the mention. */
export const hoverCardMentionOpen = defineExample({
    of: HoverCard,
    name: "mention-open",
    description: "the mention's profile card open below the mention",
    render: () => (
        <HoverCard defaultOpen>
            <HoverCardTrigger href="/people/ada">@ada</HoverCardTrigger>
            <HoverCardContent>
                <strong>Ada Lovelace</strong>
                <p>Wrote the first published program.</p>
            </HoverCardContent>
        </HoverCard>
    ),
});
