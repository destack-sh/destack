import * as style from "@destack/style";
import { space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit } from "@destack/view";

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
export interface LabelProperties extends Omit<JSX.LabelHTMLAttributes<HTMLLabelElement>, "class"> {
    /** The StyleX styles applied after the label's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a native label that names its control. */
export function Label(properties: LabelProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <label
            data-slot="label"
            {...rest}
            {...style.attributes([text.callout, styles.label, properties.xstyle], properties.style)}
        />
    );
}
