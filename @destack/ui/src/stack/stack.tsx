import { SPACE } from "../position/index.ts";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The styles of a stack. */
const styles = style.create({
    stack: {
        display: "flex",
        flexDirection: "column",
        justifyContent: "flex-start",
        minWidth: 0,
    },
    space: (value: string) => ({ "--stack-space": value, gap: value }),
    recursive: (value: string) => ({ "--stack-space": value, gap: 0 }),
});

/** The properties people change in a stack's controls and inspector. */
const STACK_PROPERTIES = {
    space: SPACE.described(t`The space between the stacked elements.`).default("4"),
    recursive: field
        .boolean()
        .labeled(t.context("label")`Recursive`)
        .described(t`Puts the space between the elements nested inside the stacked ones too.`)
        .default(false),
};

/** The properties of a stack besides its declared ones: its native attributes. */
export interface StackAttributes
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The number of elements after which the rest move to the end of the stack. */
    readonly splitAfter?: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;
}

/** Stack elements in a column with one space between each. */
export const Stack = defineComponent({
    name: "stack",
    properties: STACK_PROPERTIES,
    render(properties: RenderProperties<StackAttributes, typeof STACK_PROPERTIES>): JSX.Element {
        return renderPart(
            "div",
            "stack",
            omit(properties, "space", "recursive", "splitAfter"),
            () => [
                styles.stack,
                properties.recursive
                    ? styles.recursive(space[properties.space])
                    : styles.space(space[properties.space]),
            ],
            {
                get "data-recursive"() {
                    return properties.recursive ? "" : undefined;
                },
                get "data-split-after"() {
                    return properties.splitAfter;
                },
            },
        );
    },
});

/** The properties of a stack, its native attributes included. */
export type StackProperties = ComponentProperties<typeof Stack>;
