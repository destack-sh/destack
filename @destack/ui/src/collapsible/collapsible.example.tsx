import { defineExample } from "@destack/package/declare";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./collapsible.tsx";

/** The first tags of a note and the rest on request. */
export const collapsibleNoteTags = defineExample({
    of: Collapsible,
    name: "note-tags",
    description: "the first tags of a note and the rest on request",
    render: () => (
        <Collapsible>
            <CollapsibleTrigger>travel and 3 more</CollapsibleTrigger>
            <CollapsibleContent>food, family, summer</CollapsibleContent>
        </Collapsible>
    ),
});

/** Every tag of a note shown after the request. */
export const collapsibleNoteTagsExpanded = defineExample({
    of: Collapsible,
    name: "note-tags-expanded",
    description: "every tag of a note shown after the request",
    render: () => (
        <Collapsible open>
            <CollapsibleTrigger>travel and 3 more</CollapsibleTrigger>
            <CollapsibleContent>food, family, summer</CollapsibleContent>
        </Collapsible>
    ),
});
