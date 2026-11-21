import {
    type Accessor,
    createControllableSignal,
    createProjection,
    createSignal,
    type Setter,
} from "@destack/view";
import type { Collection } from "../collection/index.ts";

/** How a plain press selects. */
export type SelectionBehavior = "toggle" | "replace";

/** The modifier keys held during a press. */
export type SelectionModifiers = Pick<MouseEvent, "shiftKey" | "metaKey" | "ctrlKey">;

/** The selected value of a component that takes one value at a time, or none. */
export interface SingleSelection {
    /** How a plain press selects, toggling by default. */
    readonly behavior?: SelectionBehavior;
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
    /** How a plain press selects, toggling by default. */
    readonly behavior?: SelectionBehavior;
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
    /** The value a range extends from. */
    readonly #anchor: Accessor<string | undefined>;
    /** Move the value a range extends from. */
    readonly #setAnchor: Setter<string | undefined>;
    /** The value the last range extended to. */
    readonly #extended: Accessor<string | undefined>;
    /** Move the value the last range extended to. */
    readonly #setExtended: Setter<string | undefined>;

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

        // start ranges from no value
        [this.#anchor, this.#setAnchor] = createSignal<string | undefined>(undefined);
        [this.#extended, this.#setExtended] = createSignal<string | undefined>(undefined);

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

    /** Read the value a range extends from. */
    anchor(): string | undefined {
        return this.#anchor();
    }

    /** Select a value by a press. */
    select<Item>(value: string, held: SelectionModifiers, collection: Collection<Item>): void {
        // extend a multiple selection's range with shift
        if (held.shiftKey && this.isMultiple()) {
            this.extend(value, collection);
        }
        // toggle with command or control, or on a plain press under the toggle behavior
        else if (
            held.metaKey ||
            held.ctrlKey ||
            (this.#properties.behavior ?? "toggle") === "toggle"
        ) {
            this.toggle(value);
        }
        // replace the selection with the value under the replace behavior
        else {
            this.replace([value]);
        }
    }

    /** Select a value, or drop it once selected. */
    toggle(value: string): void {
        const values = this.values();
        this.#anchorAt(value);
        if (values.includes(value)) {
            this.#setValues(values.filter((entry) => entry !== value));
        } else {
            this.#setValues(this.isMultiple() ? [...values, value] : [value]);
        }
    }

    /** Select a value without dropping it. */
    add(value: string): void {
        const values = this.values();
        this.#anchorAt(value);
        if (!values.includes(value)) {
            this.#setValues(this.isMultiple() ? [...values, value] : [value]);
        }
    }

    /** Replace the selected values. */
    replace(values: readonly string[]): void {
        this.#anchorAt(values.length === 1 ? values[0] : undefined);
        this.#setValues(values);
    }

    /** Select from the anchor to a value in collection order. */
    extend<Item>(value: string, collection: Collection<Item>): void {
        // start the range at the value itself without an anchor
        const anchor = this.#anchor() ?? value;
        const previous = new Set(rangeOf(anchor, this.#extended() ?? anchor, collection));
        const range = rangeOf(anchor, value, collection);

        // keep the values outside the last range and add the new one
        const kept = this.values().filter((entry) => !previous.has(entry));
        const isKept = new Set(kept);
        const added = range.filter((entry) => !isKept.has(entry));
        this.#setAnchor(anchor);
        this.#setExtended(value);
        this.#setValues([...kept, ...added]);
    }

    /** Select every available value of a collection. */
    selectAll<Item>(collection: Collection<Item>): void {
        this.replace(collection.keys().filter((key) => !collection.isDisabled(key)));
    }

    /** Drop every selected value. */
    clear(): void {
        this.replace([]);
    }

    /** Anchor ranges at a value, ending the last range. */
    #anchorAt(value: string | undefined): void {
        this.#setAnchor(value);
        this.#setExtended(undefined);
    }
}

/** List the available keys from one key to another in collection order, both included. */
function rangeOf<Item>(from: string, to: string, collection: Collection<Item>): string[] {
    // order the two ends by place, an unknown end ranging over the other alone
    const start = collection.index(from) ?? collection.index(to);
    const end = collection.index(to) ?? start;
    if (start === undefined || end === undefined) {
        return [];
    }

    return collection
        .keys()
        .slice(Math.min(start, end), Math.max(start, end) + 1)
        .filter((key) => !collection.isDisabled(key));
}

/** Read a selection's value as the list of selected values. */
export function valuesOf(value: string | readonly string[] | undefined): readonly string[] {
    if (value === undefined) {
        return [];
    }

    return typeof value === "string" ? [value] : value;
}
