import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The least height of a cover and the space between and around its elements unless set: the viewport's height. */
const DEFAULTS = { minHeight: "100svh", space: "4", noPad: false } as const;

/** The styles of a cover and its centered element. */
const styles = style.create({
    cover: {
        display: "flex",
        flexDirection: "column",
    },
    layout: (minHeight: string, gap: string, padding: string) => ({
        minBlockSize: minHeight,
        gap,
        padding,
    }),
    center: {
        marginBlock: "auto",
    },
});

/** The properties of a cover, the native element's attributes included. */
export interface CoverProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The least height of the cover, any CSS length. */
    readonly minHeight?: string;
    /** The space between the centered element and those above and below it, and inside the cover's edges, a spacing step. */
    readonly space?: keyof typeof space;
    /** Whether the cover leaves out the space inside its edges. */
    readonly noPad?: boolean;
}

/** Fill at least a height, centering one element vertically with others above and below it. */
export function Cover(properties: CoverProperties): JSX.Element {
    const cover = merge(DEFAULTS, properties);

    return renderPart("div", "cover", omit(cover, "minHeight", "space", "noPad"), () => [
        styles.cover,
        styles.layout(cover.minHeight, space[cover.space], cover.noPad ? "0" : space[cover.space]),
    ]);
}

/** The properties of a cover's centered element, the native element's attributes included. */
export type CoverCenterProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** Render the element a cover centers vertically, such as a page's heading. */
export function CoverCenter(properties: CoverCenterProperties): JSX.Element {
    return renderPart("div", "cover-center", properties, styles.center);
}
