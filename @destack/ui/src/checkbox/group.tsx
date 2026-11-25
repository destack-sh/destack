import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createControllableSignal,
    type JSX,
    merge,
    omit,
    untrack,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";
import { useFieldValue } from "../field/control.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import type { CheckboxProperties } from "./checkbox.tsx";

/** The styles of a group of checkboxes. */
const styles = style.create({
    group: {
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-start",
        gap: space[2],
    },
});

/** The properties of a group of checkboxes, the native element's attributes included. */
export type CheckboxGroupProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties & {
        /** The values of the checked boxes, which makes the group controlled. */
        readonly value?: readonly string[] | undefined;
        /** The values checked at first while uncontrolled. */
        readonly defaultValue?: readonly string[] | undefined;
        /** Handle the person checking or unchecking a box. */
        readonly onValueChange?: ((value: readonly string[]) => void) | undefined;
        /** Every value the parent box checks, those of the boxes shown by default. */
        readonly allValues?: readonly string[] | undefined;
        /** The form field name each checked box submits its value under. */
        readonly name?: string | undefined;
        /** Whether every box ignores the person. */
        readonly disabled?: boolean | undefined;
        /** Whether the person reads the boxes but does not change them. */
        readonly readOnly?: boolean | undefined;
    };

/** The checked values of a group of checkboxes, which its boxes share. */
export class CheckboxGroupControl {
    /** The group's properties. */
    readonly #properties: CheckboxGroupProperties;
    /** The checked values. */
    readonly #values: () => readonly string[];
    /** Replace the checked values and tell the owner. */
    readonly #setValues: (values: readonly string[]) => void;
    /** The values of the boxes shown, each with whether it takes no changes. */
    readonly #boxes: CollectionBuilder<{
        readonly value: string;
        readonly isDisabled: () => boolean;
    }>;

    /** Follow a group's checked values, controlled or its own. */
    constructor(properties: CheckboxGroupProperties) {
        // follow the owner's values, else the group's own
        const [values, setValues] = createControllableSignal<readonly string[]>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? [],
            defaultValue: untrack(() => properties.defaultValue) ?? [],
            onChange: (next) => properties.onValueChange?.(next),
        });
        this.#properties = properties;
        this.#values = values;
        this.#setValues = setValues;
        this.#boxes = new CollectionBuilder();
    }

    /** Read the properties of a box of the group, those of its parent box included. */
    checkbox(given: CheckboxProperties): CheckboxProperties {
        // follow its value among the others
        const group = this.#properties;
        const isParent = untrack(() => given.parent === true);
        const value = untrack(() => given.value ?? given.name ?? "");
        const isDisabled = (): boolean => given.disabled === true || group.disabled === true;
        if (!isParent) {
            this.#boxes.add({ value, isDisabled });
        }

        // read it checked where its value is, and the parent where every value is
        const isChecked = (): boolean =>
            isParent
                ? this.all().every((each) => this.#values().includes(each)) && this.all().length > 0
                : this.#values().includes(value);
        const isPartial = (): boolean => this.isPartial();

        // check the box's value, or every one by the parent's
        return merge(given, {
            get name() {
                return isParent ? undefined : group.name;
            },
            get checked() {
                return isChecked();
            },
            get indeterminate() {
                return isParent && !isChecked() && isPartial();
            },
            get disabled() {
                return isDisabled();
            },
            get readOnly() {
                return given.readOnly === true || group.readOnly === true;
            },
            onCheckedChange: (checked: boolean) => {
                given.onCheckedChange?.(checked);
                if (isParent) {
                    this.toggleAll(checked);
                } else {
                    this.toggle(value, checked);
                }
            },
        });
    }

    /** List every value the parent box checks. */
    all(): readonly string[] {
        return this.#properties.allValues ?? this.#boxes.items().map(({ value }) => value);
    }

    /** Report whether some values are checked, though perhaps not all. */
    isPartial(): boolean {
        return this.all().some((value) => this.#values().includes(value));
    }

    /** Check or uncheck a value. */
    toggle(value: string, checked: boolean): void {
        const others = this.#values().filter((each) => each !== value);
        this.#setValues(checked ? [...others, value] : others);
    }

    /** Check or uncheck every value, those of boxes that take no changes left as they are. */
    toggleAll(checked: boolean): void {
        // take the values that change, in the group's order
        const fixed = new Set(
            this.#boxes
                .items()
                .filter(({ isDisabled }) => isDisabled())
                .map(({ value }) => value),
        );
        const all = this.all();
        const isOn = (value: string): boolean =>
            fixed.has(value) ? this.#values().includes(value) : checked;
        const others = this.#values().filter((value) => !all.includes(value));
        this.#setValues([...others, ...all.filter(isOn)]);
    }
}

/** The group of checkboxes around, null outside one. */
export const CheckboxGroupContext = createContext<CheckboxGroupControl | null>(null);

/** Render a group of checkboxes that share their checked values. */
export function CheckboxGroup(given: CheckboxGroupProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onValueChange",
        read: (value) => (Array.isArray(value) ? value.map(String) : []),
        settles: true,
    });
    const control = new CheckboxGroupControl(properties);
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "allValues",
        "name",
        "disabled",
        "readOnly",
    );

    return (
        <CheckboxGroupContext value={control}>
            {renderPart("div", "checkbox-group", rest, styles.group, {
                role: "group" as const,
                get "data-disabled"() {
                    return properties.disabled === true ? "" : undefined;
                },
            })}
        </CheckboxGroupContext>
    );
}
