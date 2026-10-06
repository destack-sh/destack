import {
    createContext,
    createEffect,
    createSignal,
    createUniqueId,
    useContext,
    type Accessor,
    type Setter,
} from "@destack/view";

/** The control of the nearest field, null outside a field. */
export const FieldContext = createContext<FieldControl | null>(null);

/** The control a field labels, describes and validates. */
export class FieldControl {
    /** The id of the control element. */
    readonly id: string;
    /** Whether the field's value is invalid. */
    readonly isInvalid: Accessor<boolean>;
    /** Whether the field is disabled. */
    readonly isDisabled: Accessor<boolean>;
    /** The ids of the descriptions and errors on screen, in the order they appeared. */
    readonly descriptions: Accessor<readonly string[]>;
    /** Replace the ids of the descriptions and errors on screen. */
    readonly #setDescriptions: Setter<readonly string[]>;

    /** Create the control of a field with a fresh id. */
    constructor(isInvalid: Accessor<boolean>, isDisabled: Accessor<boolean>) {
        // start without descriptions under a fresh id
        const [descriptions, setDescriptions] = createSignal<readonly string[]>([]);
        this.id = createUniqueId();
        this.isInvalid = isInvalid;
        this.isDisabled = isDisabled;
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
export function useFieldControl(): FieldControl | null {
    return useContext(FieldContext);
}
