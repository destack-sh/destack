import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge } from "@destack/view";
import { useFieldControl } from "../field/control.ts";
import { inputStyle } from "../input/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

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
export type TextareaProperties = Omit<JSX.TextareaHTMLAttributes<HTMLTextAreaElement>, "class"> &
    ElementPartProperties;

/** Render a native textarea that grows with its content, tied to its field. */
export function Textarea(properties: TextareaProperties): JSX.Element {
    // take the id, descriptions and state of the nearest field
    const field = useFieldControl();
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return renderPart(
        "textarea",
        "textarea",
        properties,
        () => [inputStyle({ invalid: isInvalid() }), styles.textarea],
        merge(() => field?.attributes() ?? {}),
    );
}
