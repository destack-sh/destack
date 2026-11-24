import { Shortcut } from "@destack/locale";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge } from "@destack/view";
import { readText, takeText, useFieldControl, useFieldValue } from "../field/control.ts";
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
export function Textarea(given: TextareaProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onInput",
        read: readText,
        take: takeText,
    });

    // take the id, descriptions and state of the nearest field
    const field = useFieldControl();
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return renderPart(
        "textarea",
        "textarea",
        properties,
        () => [inputStyle({ invalid: isInvalid() }), styles.textarea],
        merge(() => field?.attributes() ?? {}, { onKeyDown: submit }),
    );
}

/** Submit a text area's form by the command key and Enter, which alone starts a line. */
function submit(event: KeyboardEvent & { readonly currentTarget: HTMLTextAreaElement }): void {
    // leave Enter while a character is composed
    const isCommand = Shortcut.isCommand(navigator.userAgent);
    const isTaken = event.isComposing || event.defaultPrevented;
    if (Shortcut.matches("mod+enter", event, isCommand) && !isTaken) {
        event.preventDefault();
        event.currentTarget.form?.requestSubmit();
    }
}
