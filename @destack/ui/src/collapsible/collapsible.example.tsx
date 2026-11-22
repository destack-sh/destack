import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space } from "@destack/theme/tokens.stylex";
import { buttonVariants } from "../button/index.ts";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./collapsible.tsx";

/** The styles of the tags a note shows on request. */
const styles = style.create({
    rest: {
        paddingBlock: space[2],
        paddingInline: space[2],
        color: color.mutedForeground,
    },
});

/** The look of the request: a small quiet button. */
const TRIGGER = buttonVariants({ variant: "ghost", size: "sm" });

/** The first tags of a note and the rest on request. */
export const collapsibleNoteTags = defineExample({
    of: Collapsible,
    name: "note-tags",
    description: "the first tags of a note and the rest on request",
    render: () => (
        <Collapsible>
            <CollapsibleTrigger xstyle={TRIGGER}>travel and 3 more</CollapsibleTrigger>
            <CollapsibleContent xstyle={[text.sm, styles.rest]}>
                food, family, summer
            </CollapsibleContent>
        </Collapsible>
    ),
});

/** Every tag of a note shown after the request. */
export const collapsibleNoteTagsExpanded = defineExample({
    of: Collapsible,
    name: "note-tags-expanded",
    description: "every tag of a note shown after the request",
    properties: {
        open: true,
    },
    render: (properties) => (
        <Collapsible {...properties}>
            <CollapsibleTrigger xstyle={TRIGGER}>travel and 3 more</CollapsibleTrigger>
            <CollapsibleContent xstyle={[text.sm, styles.rest]}>
                food, family, summer
            </CollapsibleContent>
        </Collapsible>
    ),
});

/** The tags of a note whose request for the rest is unavailable. */
export const collapsibleNoteTagsDisabled = defineExample({
    of: Collapsible,
    name: "note-tags-disabled",
    description: "the tags of a note whose request for the rest is unavailable",
    properties: {
        disabled: true,
    },
    render: (properties) => (
        <Collapsible {...properties}>
            <CollapsibleTrigger xstyle={TRIGGER}>travel and 3 more</CollapsibleTrigger>
            <CollapsibleContent xstyle={[text.sm, styles.rest]}>
                food, family, summer
            </CollapsibleContent>
        </Collapsible>
    ),
});
