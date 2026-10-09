import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The widest a centered column grows and its gutters unless set, a readable line of 65 characters. */
const DEFAULTS = { max: "65ch", gutters: "4", intrinsic: false, andText: false } as const;

/** The styles of a centered column. */
const styles = style.create({
    center: {
        boxSizing: "content-box",
        marginInline: "auto",
    },
    intrinsic: {
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
    },
    text: {
        textAlign: "center",
    },
    layout: (max: string, gutters: string) => ({
        maxInlineSize: max,
        paddingInline: gutters,
    }),
});

/** The properties of a centered column, the native element's attributes included. */
export interface CenterProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The widest the column grows, any CSS length. */
    readonly max?: string;
    /** The space kept on each side at narrow widths, a spacing step. */
    readonly gutters?: keyof typeof space;
    /** Whether the elements inside center at their own widths too. */
    readonly intrinsic?: boolean;
    /** Whether the text inside centers too. */
    readonly andText?: boolean;
}

/** Center a column in its container up to a maximum width, with gutters at narrow widths. */
export function Center(properties: CenterProperties): JSX.Element {
    const center = merge(DEFAULTS, properties);

    return renderPart(
        "div",
        "center",
        omit(center, "max", "gutters", "intrinsic", "andText"),
        () => [
            styles.center,
            center.intrinsic && styles.intrinsic,
            center.andText && styles.text,
            styles.layout(center.max, space[center.gutters]),
        ],
    );
}
