import * as style from "@destack/style";
import { space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of a label. */
const styles = style.create({
    label: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        fontWeight: weight.medium,
        lineHeight: 1,
        userSelect: "none",
    },
});

/** The properties of a label, the native label's attributes included. */
export type LabelProperties = Omit<JSX.LabelHTMLAttributes<HTMLLabelElement>, "class"> &
    ElementPartProperties;

/** Render a native label that names its control. */
export function Label(properties: LabelProperties): JSX.Element {
    return renderPart("label", "label", properties, [text.callout, styles.label]);
}
