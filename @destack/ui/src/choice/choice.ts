import { type Accessor, createControllableSignal } from "@destack/view";

/** The chosen value of a component that takes one value at a time, or none. */
export interface SingleChoice {
    /** One value is chosen at a time, the default. */
    readonly multiple?: false;
    /** The chosen value, which makes the choice controlled, undefined for none. */
    readonly value?: string | undefined;
    /** The value chosen at first while uncontrolled. */
    readonly defaultValue?: string;
    /** Handle the chosen value changing, undefined once none is chosen. */
    readonly onValueChange?: (value: string | undefined) => void;
}

/** The chosen values of a component that takes several values at a time. */
export interface MultipleChoice {
    /** Several values are chosen at a time. */
    readonly multiple: true;
    /** The chosen values, which make the choice controlled. */
    readonly value?: readonly string[];
    /** The values chosen at first while uncontrolled. */
    readonly defaultValue?: readonly string[];
    /** Handle the chosen values changing. */
    readonly onValueChange?: (value: readonly string[]) => void;
}

/** The choice of a component that takes one value or several. */
export type Choice = SingleChoice | MultipleChoice;

/** Follow a component's chosen values, controlled or its own, as a list: one value or none for a single choice. */
export function createChoice(
    choice: Choice,
): [Accessor<readonly string[]>, (values: readonly string[]) => void] {
    return createControllableSignal<readonly string[]>({
        isControlled: () => "value" in choice,
        value: () => valuesOf(choice.value),
        defaultValue: valuesOf(choice.defaultValue),
        onChange: (next) => {
            // report a single choice as its one value and a multiple one as its list
            if (choice.multiple === true) {
                choice.onValueChange?.(next);
            } else {
                choice.onValueChange?.(next[0]);
            }
        },
    });
}

/** Choose a value: alone in a single choice, beside the others in a multiple one, or drop it once chosen. */
export function toggled(
    choice: Choice,
    values: readonly string[],
    value: string,
): readonly string[] {
    if (values.includes(value)) {
        return values.filter((entry) => entry !== value);
    }

    return choice.multiple === true ? [...values, value] : [value];
}

/** Read a choice's value as the list of chosen values. */
export function valuesOf(value: string | readonly string[] | undefined): readonly string[] {
    if (value === undefined) {
        return [];
    }

    return typeof value === "string" ? [value] : value;
}
