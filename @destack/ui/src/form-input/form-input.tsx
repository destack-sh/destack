import * as style from "@destack/style";
import { createEffect, type JSX, onSettled } from "@destack/view";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The styles of the input of a control's value. */
const styles = style.create({
    input: {
        pointerEvents: "none",
    },
});

/** The form a control's value submits with. */
export interface FormInputProperties {
    /** The form field name the value submits under. */
    readonly name?: string | undefined;
    /** The id of the form the control belongs to, its ancestor form by default. */
    readonly form?: string | undefined;
    /** Whether a form needs a value. */
    readonly required?: boolean | undefined;
}

/** Report whether a control submits to a form, which needs the input of its value. */
export function isFormControl(properties: FormInputProperties): boolean {
    return (
        properties.name !== undefined ||
        properties.required === true ||
        properties.form !== undefined
    );
}

/** Run as the form of a control resets. */
export function handleFormReset(
    form: () => HTMLFormElement | null | undefined,
    onReset: () => void,
): void {
    const reset = (event: Event): void => {
        if (event.target === form()) {
            onReset();
        }
    };
    onSettled(() => {
        document.addEventListener("reset", reset);

        return () => document.removeEventListener("reset", reset);
    });
}

/** Render the input that submits and validates a control's value with its form. */
export function FormInput(
    properties: FormInputProperties & {
        /** The value as the form submits it, empty for none. */
        readonly value: string;
        /** Why the value is refused, empty while it is not. */
        readonly error: string;
        /** Whether the control is left out of its form. */
        readonly disabled: boolean;
        /** Return the control to its first value when the form resets. */
        readonly onReset: () => void;
        /** Focus the control, as the form asks for a missing value. */
        readonly onFocus: () => void;
    },
): JSX.Element {
    // reset with the form, and refuse what the control refuses
    let element: HTMLInputElement | undefined;
    handleFormReset(
        () => element?.form,
        () => properties.onReset(),
    );
    createEffect(
        () => properties.error,
        (error) => element?.setCustomValidity(error),
    );

    return (
        <input
            type="text"
            aria-hidden="true"
            tabindex={-1}
            name={properties.name}
            form={properties.form}
            required={properties.required}
            disabled={properties.disabled}
            value={properties.value}
            ref={(input) => (element = input)}
            onFocus={() => properties.onFocus()}
            {...style.attributes([visuallyHiddenStyle(), styles.input])}
        />
    );
}
