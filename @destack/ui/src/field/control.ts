import {
    createContext,
    createEffect,
    createSignal,
    createUniqueId,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";

/** The control of the nearest field, null outside a field. */
export const FieldContext = createContext<FieldControl | null>(null);

/** A value a field's owner shows in the field's control and commits from it, such as an object field. */
export interface FieldValue {
    /** Write the value as a text control shows it. */
    text(): string;
    /** Report whether an on and off control shows the value as on. */
    isChecked(): boolean;
    /** Take the input a person committed: a control's text, or an on and off control's state. */
    commit(raw: string | boolean): void;
}

/** The control a field labels, describes and validates, showing the field's value when it has one. */
export class FieldControl {
    /** The id of the control element. */
    readonly id: string;
    /** Whether the field's value is invalid. */
    readonly isInvalid: Accessor<boolean>;
    /** Whether the field is disabled. */
    readonly isDisabled: Accessor<boolean>;
    /** The value the field's owner shows and commits, undefined for a control that keeps its own. */
    readonly value: Accessor<FieldValue | undefined>;
    /** The ids of the descriptions and errors on screen, in the order they appeared. */
    readonly descriptions: Accessor<readonly string[]>;
    /** Replace the ids of the descriptions and errors on screen. */
    readonly #setDescriptions: Setter<readonly string[]>;

    /** Create the control of a field with a fresh id. */
    constructor(
        isInvalid: Accessor<boolean>,
        isDisabled: Accessor<boolean>,
        value: Accessor<FieldValue | undefined> = () => undefined,
    ) {
        // start without descriptions under a fresh id
        const [descriptions, setDescriptions] = createSignal<readonly string[]>([]);
        this.id = createUniqueId();
        this.isInvalid = isInvalid;
        this.isDisabled = isDisabled;
        this.value = value;
        this.descriptions = descriptions;
        this.#setDescriptions = setDescriptions;
    }

    /** Describe the control by an element's id while the element is on screen. */
    describe(id: string, isShown: Accessor<boolean>): void {
        createEffect(isShown, (shown) => {
            // leave a hidden element out
            if (!shown) {
                return undefined;
            }

            // add the id until the element hides or unmounts
            this.#setDescriptions((ids) => [...ids, id]);

            return () => this.#setDescriptions((ids) => ids.filter((entry) => entry !== id));
        });
    }

    /** Return the attributes that connect a control element to its field. */
    attributes(): FieldControlAttributes {
        const descriptions = this.descriptions();

        return {
            id: this.id,
            "aria-describedby": descriptions.length > 0 ? descriptions.join(" ") : undefined,
            "aria-invalid": this.isInvalid() ? "true" : undefined,
            disabled: this.isDisabled() ? true : undefined,
        };
    }

    /** Return the value and change handler a text control takes from the field's value, none without one. */
    valueAttributes<
        Target extends HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
    >(): FieldValueAttributes<Target> {
        const value = this.value();

        return value === undefined
            ? {}
            : { value: value.text(), onChange: (event) => value.commit(event.currentTarget.value) };
    }

    /** Return the checked state and change handler an on and off control takes from the field's value, none without one. */
    checkAttributes(): FieldCheckAttributes {
        const value = this.value();

        return value === undefined
            ? {}
            : {
                  checked: value.isChecked(),
                  onChange: (event) => value.commit(event.currentTarget.checked),
              };
    }
}

/** The value and change handler of a text control showing a field's value. */
export interface FieldValueAttributes<
    Target extends HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
> {
    /** The value as the control shows it. */
    readonly value?: string;
    /** Commit the value the person entered. */
    readonly onChange?: (event: Event & { readonly currentTarget: Target }) => void;
}

/** The checked state and change handler of an on and off control showing a field's value. */
export interface FieldCheckAttributes {
    /** Whether the control is on. */
    readonly checked?: boolean;
    /** Commit the state the person chose. */
    readonly onChange?: (event: Event & { readonly currentTarget: HTMLInputElement }) => void;
}

/** The attributes that connect a control element to its field. */
export interface FieldControlAttributes {
    /** The id the field's label points at. */
    readonly id: string;
    /** The ids of the field's descriptions and errors on screen. */
    readonly "aria-describedby": string | undefined;
    /** Whether the field's value is invalid. */
    readonly "aria-invalid": "true" | undefined;
    /** Whether the field is disabled. */
    readonly disabled: true | undefined;
}

/** Read the control of the nearest field, null outside a field. */
export function useField(): FieldControl | null {
    return useContext(FieldContext);
}
