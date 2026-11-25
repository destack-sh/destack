import {
    type Accessor,
    createControllableSignal,
    createProjection,
    createSignal,
    type Setter,
    untrack,
} from "@destack/view";
import type { Collection } from "../collection/index.ts";

/** How a plain press selects. */
export type SelectionBehavior = "toggle" | "replace";

/** The modifier keys down during a press. */
export type SelectionModifiers = Pick<MouseEvent, "shiftKey" | "metaKey" | "ctrlKey">;

/** The selected value of a component that takes one value at a time, or none. */
export interface SingleSelection {
    /** How a plain press selects, toggling by default. */
    readonly behavior?: SelectionBehavior;
    /** Whether a press leaves the last selected value, as React Aria's segmented controls do. */
    readonly disallowEmptySelection?: boolean;
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
    /** Whether a press leaves the last selected value, as React Aria's segmented controls do. */
    readonly disallowEmptySelection?: boolean;
    /** Several values are selected at a time. */
    readonly multiple: true;
    /** The selected values, which make the selection controlled. */
    readonly value?: readonly string[];
    /** The values selected at first while uncontrolled. */
    readonly defaultValue?: readonly string[];
    /** Handle the selected values changing. */
    readonly onValueChange?: (value: readonly string[]) => void;
}

/** The names of a selection's properties, which a component leaves off its element. */
export const SELECTION_OPTIONS = [
    "multiple",
    "value",
    "defaultValue",
    "onValueChange",
    "behavior",
    "disallowEmptySelection",
] as const;

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
            defaultValue: untrack(() => valuesOf(properties.defaultValue)),
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

    /** Read the value a range extends from, none once the owner dropped it from the selection. */
    anchor(): string | undefined {
        const anchor = this.#anchor();

        return anchor !== undefined && this.isSelected(anchor) ? anchor : undefined;
    }

    /** List what an action on an item takes: the selection it is in, in order, else the item. */
    targets<Item>(value: string, collection: Collection<Item>): readonly string[] {
        return this.isSelected(value)
            ? collection.keys().filter((key) => this.isSelected(key))
            : [value];
    }

    /** Report whether a press selects: with a modifier, or always where no press opens. */
    isSelecting(
        press: SelectionModifiers & { readonly pointerType: string },
        hasAction: boolean,
    ): boolean {
        const isModified = press.metaKey || press.ctrlKey || press.shiftKey;

        return press.pointerType !== "keyboard" && (isModified || !hasAction);
    }

    /** Select a value by a press. */
    select<Item>(value: string, modifiers: SelectionModifiers, collection: Collection<Item>): void {
        // extend a multiple selection's range with shift
        if (modifiers.shiftKey && this.isMultiple()) {
            this.extend(value, collection);
        }
        // toggle with command or control, or on a plain press under the toggle behavior
        else if (
            modifiers.metaKey ||
            modifiers.ctrlKey ||
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
        // read what dropping the value would leave
        const values = this.values();
        const rest = values.filter((entry) => entry !== value);
        const isLast = rest.length === 0 && this.#properties.disallowEmptySelection === true;
        this.#anchorAt(value);

        // leave the last value where the selection may not be empty
        if (values.includes(value) && isLast) {
            return;
        }
        // drop a selected value
        else if (values.includes(value)) {
            this.#setValues(rest);
        }
        // add the value, in place of the others in a single selection
        else {
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
        this.#range(this.anchor(), value, collection);
    }

    /** Extend by a step from a focused value, anchoring there when nothing anchors a range. */
    extendFrom<Item>(from: string, value: string, collection: Collection<Item>): void {
        this.#range(this.anchor() ?? from, value, collection);
    }

    /** Select from an anchor to a value, replacing the anchor's last range. */
    #range<Item>(start: string | undefined, value: string, collection: Collection<Item>): void {
        // start the range at the value itself without an anchor
        const anchor = start ?? value;
        const extended =
            start === undefined || start !== this.anchor() ? undefined : this.#extended();
        const previous = new Set(rangeOf(anchor, extended ?? anchor, collection));
        const range = rangeOf(anchor, value, collection);

        // leave the values outside the last range and add the new one
        const rest = this.values().filter((entry) => !previous.has(entry));
        const isOutside = new Set(rest);
        const added = range.filter((entry) => !isOutside.has(entry));
        this.#setAnchor(anchor);
        this.#setExtended(value);
        this.#setValues([...rest, ...added]);
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
