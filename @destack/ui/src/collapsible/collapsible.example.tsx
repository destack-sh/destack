import type { JSX } from "@solidjs/web";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./collapsible.tsx";

/** Show the first tags of a note and the rest on request. */
export function CollapsibleExample(): JSX.Element {
    return (
        <Collapsible>
            <CollapsibleTrigger>travel and 3 more</CollapsibleTrigger>
            <CollapsibleContent>food, family, summer</CollapsibleContent>
        </Collapsible>
    );
}
