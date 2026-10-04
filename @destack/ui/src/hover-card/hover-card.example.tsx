import type { JSX } from "@solidjs/web";
import { HoverCard, HoverCardContent, HoverCardTrigger } from "./hover-card.tsx";

/** Show a person's profile while the pointer rests on a mention. */
export function HoverCardExample(): JSX.Element {
    return (
        <HoverCard>
            <HoverCardTrigger href="/people/ada">@ada</HoverCardTrigger>
            <HoverCardContent>
                <strong>Ada Lovelace</strong>
                <p>Wrote the first published program.</p>
            </HoverCardContent>
        </HoverCard>
    );
}
