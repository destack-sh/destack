import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space } from "@destack/theme/tokens.stylex";
import { Button } from "../button/index.ts";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./collapsible.tsx";

/** The styles of the tags a note shows on request. */
const styles = style.create({
    rest: {
        paddingBlock: space[2],
        paddingInline: space[2],
        color: color.mutedForeground,
    },
});

/** The first tags of a note and the rest on request. */
export const collapsibleNoteTags = defineExample({
    of: Collapsible,
    name: "noteTags",
    description: t`The first tags of a note and the rest on request.`,
    render: (properties) => (
        <Collapsible {...properties}>
            <CollapsibleTrigger
                render={(attributes) => <Button {...attributes} variant="ghost" size="sm" />}
            >
                travel and 3 more
            </CollapsibleTrigger>
            <CollapsibleContent xstyle={[text.sm, styles.rest]}>
                food, family, summer
            </CollapsibleContent>
        </Collapsible>
    ),
});

/** Every tag of a note shown after the request. */
export const collapsibleContentDefault = defineExample({
    of: CollapsibleContent,
    name: "default",
    description: t`Every tag of a note shown after the request.`,
    render: (properties) => (
        <Collapsible open>
            <CollapsibleTrigger
                render={(attributes) => <Button {...attributes} variant="ghost" size="sm" />}
            >
                travel and 3 more
            </CollapsibleTrigger>
            <CollapsibleContent xstyle={[text.sm, styles.rest]} {...properties}>
                food, family, summer
            </CollapsibleContent>
        </Collapsible>
    ),
});
