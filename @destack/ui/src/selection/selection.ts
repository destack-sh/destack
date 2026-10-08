import { type Accessor, createControllableSignal, createProjection } from "@destack/view";

/** The selected value of a component that takes one value at a time, or none. */
export interface SingleSelection {
    /** One value is selected at a time, the default. */
    readonly multiple?: false;
    /** The selected value, which makes the selection controlled, undefined for none. */
    readonly value?: string | undefined;
    /** The value selected at first while uncontrolled. */
    readonly defaultValue?: string;
    /** Handle the selected value changing, undefined once none is selected. */
    readonly onValueChange?: (value: string | undefined) => void;
}

/** The selected values of a component that takes several values at a time. */
export interface MultipleSelection {
    /** Several values are selected at a time. */
    readonly multiple: true;
    /** The selected values, which make the selection controlled. */
    readonly value?: readonly string[];
    /** The values selected at first while uncontrolled. */
    readonly defaultValue?: readonly string[];
    /** Handle the selected values changing. */
    readonly onValueChange?: (value: readonly string[]) => void;
}

/** The selection properties of a component that takes one value or several. */
export type SelectionProperties = SingleSelection | MultipleSelection;

/** The selected values of a collection, controlled or its own. */
export class Selection {
    /** The selected values: one or none for a single selection. */
    readonly values: Accessor<readonly string[]>;
    /** The selection properties the selection follows. */
    readonly #properties: SelectionProperties;
    /** Whether each value is selected, which only the values that change read anew. */
    readonly #selected: Readonly<Record<string, boolean>>;
    /** Replace the selected values and tell the owner. */
    readonly #setValues: (values: readonly string[]) => void;

    /** Follow a component's selection properties. */
    constructor(properties: SelectionProperties) {
        // follow the controlled values or the selection's own, reporting a single selection's one value
        const [values, setValues] = createControllableSignal<readonly string[]>({
            isControlled: () => "value" in properties,
            value: () => valuesOf(properties.value),
            defaultValue: valuesOf(properties.defaultValue),
            onChange: (next) => {
                if (properties.multiple === true) {
                    properties.onValueChange?.(next);
                } else {
                    properties.onValueChange?.(next[0]);
                }
            },
        });
        this.values = values;
        this.#properties = properties;
        this.#setValues = setValues;

        // mark each selected value, so a change reruns only the items it flips
        this.#selected = createProjection<Record<string, boolean>>((draft) => {
            const selected = new Set(values());
            for (const name of Object.keys(draft)) {
                if (!selected.has(name)) {
                    delete draft[name];
                }
            }
            for (const value of selected) {
                draft[value] = true;
            }
        }, {});
    }

    /** Report whether several values are selected at a time. */
    isMultiple(): boolean {
        return this.#properties.multiple === true;
    }

    /** Report whether a value is selected. */
    isSelected(value: string): boolean {
        return this.#selected[value] === true;
    }

    /** Select a value, alone in a single selection and beside the others in a multiple one, or drop it once selected. */
    toggle(value: string): void {
        const values = this.values();
        if (values.includes(value)) {
            this.#setValues(values.filter((entry) => entry !== value));
        } else {
            this.#setValues(this.isMultiple() ? [...values, value] : [value]);
        }
    }

    /** Select a value without dropping it: alone in a single selection and beside the others in a multiple one. */
    select(value: string): void {
        const values = this.values();
        if (!values.includes(value)) {
            this.#setValues(this.isMultiple() ? [...values, value] : [value]);
        }
    }

    /** Replace the selected values. */
    replace(values: readonly string[]): void {
        this.#setValues(values);
    }
}

/** Read a selection's value as the list of selected values. */
export function valuesOf(value: string | readonly string[] | undefined): readonly string[] {
    if (value === undefined) {
        return [];
    }

    return typeof value === "string" ? [value] : value;
}
