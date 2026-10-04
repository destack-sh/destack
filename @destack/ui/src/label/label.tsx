import * as style from "@destack/style";
import { space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";

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
export interface LabelProperties extends Omit<
    JSX.LabelHTMLAttributes<HTMLLabelElement>,
    "class" | "style"
> {
    /** The StyleX styles applied after the label's styles. */
    readonly style?: style.Styles;
}

/** Render a native label that names its control. */
export function Label(properties: LabelProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <label
            data-slot="label"
            {...rest}
            {...style.attrs(text.callout, styles.label, properties.style)}
        />
    );
}
