import * as style from "@destack/style";
import { stroke } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";

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
export interface VisuallyHiddenProperties extends Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class"
> {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
}

/** Return the StyleX styles that take an element out of sight and leave it to assistive technology. */
export function visuallyHiddenStyle(): style.Styles {
    return styles.hidden;
}

/** Render content out of sight that assistive technology still reads, such as an icon button's name. */
export function VisuallyHidden(properties: VisuallyHiddenProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <span
            data-slot="visually-hidden"
            {...rest}
            {...style.attributes([styles.hidden, properties.xstyle], properties.style)}
        />
    );
}
