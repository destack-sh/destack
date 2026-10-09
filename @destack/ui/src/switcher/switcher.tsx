import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import "./switcher.css";

/** The width below which a switcher stacks its elements, the space between them and the most it keeps in a row unless set, about 60 characters. */
const DEFAULTS = { threshold: "30rem", space: "4", limit: 4 } as const;

/** The styles of a switcher. */
const styles = style.create({
    switcher: {
        display: "flex",
        flexWrap: "wrap",
    },
    layout: (threshold: string, gap: string) => ({
        "--switcher-threshold": threshold,
        gap,
    }),
});

/** The properties of a switcher, the native element's attributes included. */
export interface SwitcherProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The container width below which the elements stack, any CSS length. */
    readonly threshold?: string;
    /** The space between the elements, a spacing step. */
    readonly space?: keyof typeof space;
    /** The most elements kept in one row, above which they always stack, 4 by default. */
    readonly limit?: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;
}

/** Lay elements out in one row of equal widths, or one column when the container is narrower than the threshold. */
export function Switcher(properties: SwitcherProperties): JSX.Element {
    const switcher = merge(DEFAULTS, properties);

    return renderPart(
        "div",
        "switcher",
        omit(switcher, "threshold", "space", "limit"),
        () => [styles.switcher, styles.layout(switcher.threshold, space[switcher.space])],
        {
            get "data-limit"() {
                return switcher.limit;
            },
        },
    );
}
