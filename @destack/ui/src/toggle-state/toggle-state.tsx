import { Timeout } from "@destack/view/primitives/timer";
import * as style from "@destack/style";
import { handleFormReset } from "../form-input/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";
import { type Accessor, createControllableSignal, type JSX, Show, untrack } from "@destack/view";

/** The styles of the hidden native input that submits a control's state with its form. */
const styles = style.create({
    input: {
        opacity: 0,
        pointerEvents: "none",
    },
});

/** The on and off state of a toggle control, controlled by passing `checked`. */
export interface ToggleStateProperties {
    /** Whether the control is checked, which makes the state controlled. */
    readonly checked?: boolean | undefined;
    /** Whether the control starts checked while uncontrolled. */
    readonly defaultChecked?: boolean | undefined;
    /** Handle the person checking or unchecking the control. */
    readonly onCheckedChange?: ((checked: boolean) => void) | undefined;
    /** Whether the person reads the state but does not change it. */
    readonly readOnly?: boolean | undefined;
}

/** The form attributes of a control, which its hidden native input submits. */
export interface ToggleFormProperties {
    /** The form field name the control submits under. */
    readonly name?: string | undefined;
    /** The value the control submits while checked, `on` by default. */
    readonly value?: string | undefined;
    /** The value the control submits while unchecked, none by default. */
    readonly uncheckedValue?: string | undefined;
    /** Whether a form needs the control checked before it submits. */
    readonly required?: boolean | undefined;
    /** The id of the form the control belongs to, its ancestor form by default. */
    readonly form?: string | undefined;
}

/** The properties of the hidden native input of a control. */
export interface ToggleInputProperties extends ToggleFormProperties {
    /** The kind of native input, a checkbox by default. */
    readonly type?: "checkbox" | "radio";
    /** Whether the input is checked. */
    readonly checked: boolean;
    /** Whether the control starts checked, which its form's reset restores. */
    readonly defaultChecked: boolean;
    /** Whether the input is disabled and left out of its form. */
    readonly disabled: boolean;
    /** Return the control to its first state when the form resets. */
    readonly onReset: () => void;
}

/** The checked state of a toggle control and the ways the person and its form change it. */
export class ToggleState {
    /** Whether the control is checked. */
    readonly isChecked: Accessor<boolean>;
    /** The properties the state follows. */
    readonly #properties: ToggleStateProperties;
    /** Replace the checked state and tell the owner. */
    readonly #setChecked: (checked: boolean) => void;

    /** Follow a toggle control's checked state, controlled or its own. */
    constructor(properties: ToggleStateProperties) {
        // follow the controlled state or the control's own
        const [isChecked, setChecked] = createControllableSignal({
            isControlled: () => properties.checked !== undefined,
            value: () => properties.checked === true,
            defaultValue: untrack(() => properties.defaultChecked) === true,
            onChange: (checked) => properties.onCheckedChange?.(checked),
        });
        this.isChecked = isChecked;
        this.#properties = properties;
        this.#setChecked = setChecked;
    }

    /** Check or uncheck the control and tell the owner, unless the person only reads it. */
    set(checked: boolean): void {
        if (this.#properties.readOnly !== true) {
            this.#setChecked(checked);
        }
    }

    /** Report whether the control starts checked. */
    isDefaultChecked(): boolean {
        return this.#properties.defaultChecked === true;
    }

    /** Return the control to the state it started in, as a form reset does. */
    reset(): void {
        this.#setChecked(this.isDefaultChecked());
    }
}

/** Report whether a control submits to a form, which needs its hidden native input. */
export function isSubmitted(properties: ToggleFormProperties): boolean {
    return (
        properties.name !== undefined ||
        properties.required === true ||
        properties.form !== undefined
    );
}

/** Render the hidden native input that submits a control's state with its form. */
export function ToggleInput(properties: ToggleInputProperties): JSX.Element {
    // restore the first state when the input's form resets, a task later on the page's clock
    let element: HTMLInputElement | undefined;
    const restoring = new Timeout();
    handleFormReset(
        () => element?.form,
        () => {
            // reset the control, then write a controlled state over the first one the form restores
            properties.onReset();
            restoring.set(() => {
                if (element !== undefined) {
                    element.checked = properties.checked;
                }
            }, 0);
        },
    );

    return (
        <>
            <input
                type={properties.type ?? "checkbox"}
                aria-hidden="true"
                tabindex={-1}
                name={properties.name}
                value={properties.value ?? "on"}
                required={properties.required}
                form={properties.form}
                disabled={properties.disabled}
                checked={properties.checked}
                defaultChecked={properties.defaultChecked}
                ref={(input) => (element = input)}
                {...style.attributes([visuallyHiddenStyle(), styles.input])}
            />
            <Show when={properties.checked ? undefined : properties.uncheckedValue}>
                {(value) => (
                    <input
                        type="hidden"
                        name={properties.name}
                        value={value()}
                        form={properties.form}
                        disabled={properties.disabled}
                    />
                )}
            </Show>
        </>
    );
}
