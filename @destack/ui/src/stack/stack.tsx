import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import "./stack.css";

/** The space between stacked elements unless set. */
const DEFAULTS = { space: "4", recursive: false } as const;

/** The styles of a stack. */
const styles = style.create({
    stack: {
        display: "flex",
        flexDirection: "column",
        justifyContent: "flex-start",
        minWidth: 0,
    },
    space: (value: string) => ({ "--stack-space": value }),
});

/** The properties of a stack, the native element's attributes included. */
export interface StackProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The space between the stacked elements, a spacing step. */
    readonly space?: keyof typeof space;
    /** Whether the space also falls between the elements nested inside the stacked ones. */
    readonly recursive?: boolean;
    /** The number of elements after which the rest move to the end of the stack. */
    readonly splitAfter?: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;
}

/** Stack elements in a column with one space between each. */
export function Stack(properties: StackProperties): JSX.Element {
    const stack = merge(DEFAULTS, properties);

    return renderPart(
        "div",
        "stack",
        omit(stack, "space", "recursive", "splitAfter"),
        () => [styles.stack, styles.space(space[stack.space])],
        {
            get "data-recursive"() {
                return stack.recursive ? "" : undefined;
            },
            get "data-split-after"() {
                return stack.splitAfter;
            },
        },
    );
}
