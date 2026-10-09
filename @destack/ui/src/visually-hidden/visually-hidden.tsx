import * as style from "@destack/style";
import { stroke } from "@destack/theme/tokens.stylex";
import { type JSX } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of an element out of sight but read by assistive technology. */
const styles = style.create({
    hidden: {
        position: "absolute",
        width: stroke.border,
        height: stroke.border,
        overflow: "hidden",
        clipPath: "inset(50%)",
        whiteSpace: "nowrap",
    },
});

/** The properties of a visually hidden element, the native span's attributes included. */
export type VisuallyHiddenProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> &
    ElementPartProperties;

/** Return the StyleX styles that take an element out of sight and leave it to assistive technology. */
export function visuallyHiddenStyle(): style.Styles {
    return styles.hidden;
}

/** Render content out of sight that assistive technology still reads, such as an icon button's name. */
export function VisuallyHidden(properties: VisuallyHiddenProperties): JSX.Element {
    return renderPart("span", "visually-hidden", properties, styles.hidden);
}
