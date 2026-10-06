import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";
import { useFieldControl } from "../field/control.ts";
import { inputStyle } from "../input/index.ts";

/** The styles of a textarea over an input's box. */
const styles = style.create({
    textarea: {
        display: "flex",
        fieldSizing: "content",
        width: "100%",
        minHeight: space[9],
        paddingBlock: space[2],
        cursor: { default: "text", ":disabled": "not-allowed" },
    },
});

/** The properties of a textarea, the native textarea's attributes included. */
export interface TextareaProperties extends Omit<
    JSX.TextareaHTMLAttributes<HTMLTextAreaElement>,
    "class"
> {
    /** The StyleX styles applied after the textarea's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a native textarea that grows with its content, tied to its field. */
export function Textarea(properties: TextareaProperties): JSX.Element {
    // take the id, descriptions and state of the nearest field
    const field = useFieldControl();
    const rest = omit(properties, "xstyle", "style");
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return (
        <textarea
            data-slot="textarea"
            {...field?.attributes()}
            {...rest}
            {...style.attributes(
                [inputStyle({ invalid: isInvalid() }), styles.textarea, properties.xstyle],
                properties.style,
            )}
        />
    );
}
