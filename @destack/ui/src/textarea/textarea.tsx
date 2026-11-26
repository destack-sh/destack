import { Shortcut } from "@destack/locale";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { readText, takeText, useFieldControl, useFieldValue } from "../field/control.ts";
import { Input, inputStyle } from "../input/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

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
    plain: {
        minHeight: "1lh",
        paddingBlock: 0,
        resize: "none",
    },
});

/** The properties of a textarea besides its declared ones: its native attributes and its part. */
export type TextareaAttributes = Omit<
    JSX.TextareaHTMLAttributes<HTMLTextAreaElement>,
    "class" | keyof typeof Input.properties
> &
    ElementPartProperties;

/** Render a native textarea that grows with its content, tied to its field. */
export const Textarea = defineComponent({
    name: "textarea",
    properties: Input.properties,
    render(given: RenderProperties<TextareaAttributes, typeof Input.properties>): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "value",
            change: "onInput",
            read: readText,
            take: takeText,
        });

        // take the id, descriptions and state of the nearest field
        const fieldControl = useFieldControl();
        const isInvalid = (): boolean =>
            fieldControl?.isInvalid() === true || properties["aria-invalid"] === "true";

        return renderPart(
            "textarea",
            "textarea",
            omit(properties, "variant"),
            () => [
                inputStyle({ variant: properties.variant, invalid: isInvalid() }),
                styles.textarea,
                properties.variant === "plain" && styles.plain,
            ],
            merge(fieldControl?.attributes() ?? {}, { onKeyDown: submit }),
        );
    },
});

/** The properties of a textarea, its native attributes included. */
export type TextareaProperties = ComponentProperties<typeof Textarea>;

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
