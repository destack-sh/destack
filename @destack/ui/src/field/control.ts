import {
    type Accessor,
    createContext,
    createSignal,
    createUniqueId,
    type FormField,
    merge,
    type Setter,
    untrack,
    useContext,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";

/** The control of the nearest field, null outside a field. */
export const FieldContext = createContext<FieldControl | null>(null);

/** The control a field labels, describes and validates. */
export class FieldControl {
    /** The id of the control element. */
    readonly id: string;
    /** The id of the field's label, which names a group of controls. */
    readonly labelId: string;
    /** Whether the field's label labels a group of controls, such as the segments of a date. */
    readonly isGrouped: Accessor<boolean>;
    /** Whether the field's value is invalid. */
    readonly isInvalid: Accessor<boolean>;
    /** Whether the field is disabled. */
    readonly isDisabled: Accessor<boolean>;
    /** The ids of the descriptions and errors on screen, in the order they appeared. */
    readonly descriptions: Accessor<readonly string[]>;
    /** Whether its label and control are side by side. */
    readonly isAcross: boolean;
    /** The form's field whose value its control edits, absent outside a form. */
    readonly value: FormField<unknown> | undefined;
    /** What the browser says is wrong with its control, empty while nothing is. */
    readonly refusal: Accessor<string>;
    /** Set what the browser says. */
    readonly #setRefusal: Setter<string>;
    /** The errors mounted in the field. */
    readonly #errors: CollectionBuilder<string>;
    /** The descriptions and errors mounted in the field. */
    readonly #descriptions: CollectionBuilder<FieldDescriptionState>;
    /** The groups of controls on screen. */
    readonly #groups: CollectionBuilder<string>;

    /** Create the control of a field with a fresh id. */
    constructor(
        isInvalid: Accessor<boolean>,
        isDisabled: Accessor<boolean>,
        value?: FormField<unknown>,
        isAcross = false,
    ) {
        // start without descriptions under a fresh id, which descriptions leave as they unmount
        this.id = createUniqueId();
        this.labelId = createUniqueId();
        this.isGrouped = () => this.#groups.size() > 0;

        // start without a refusal
        const [refusal, setRefusal] = createSignal("", { ownedWrite: true });
        this.isAcross = isAcross;
        this.value = value;
        this.refusal = refusal;
        this.#setRefusal = setRefusal;
        this.#errors = new CollectionBuilder();
        this.isInvalid = () => isInvalid() || this.problem() !== undefined;
        this.isDisabled = isDisabled;
        this.#descriptions = new CollectionBuilder();
        this.descriptions = () =>
            this.#descriptions
                .items()
                .flatMap((description) => (description.isShown() ? [description.id] : []));
        this.#groups = new CollectionBuilder();
    }

    /** Read what is wrong with the value: what its control says, else what its form does. */
    problem(): string | undefined {
        const refusal = this.refusal();

        return refusal === "" ? this.value?.problem() : refusal;
    }

    /** Show what is wrong in an element until it unmounts, in place of the browser's bubble. */
    showError(id: string): void {
        this.#errors.add(id);
    }

    /** Say what is wrong with the control, empty once nothing is. */
    say(refusal: string): void {
        this.#setRefusal(refusal);
    }

    /** Take what the browser says is wrong, and report whether the field shows it. */
    refuse(control: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement): boolean {
        // leave it to the browser without an error on screen
        if (untrack(() => this.#errors.size()) === 0) {
            return false;
        }
        this.#setRefusal(control.validationMessage);

        return true;
    }

    /** Read again what the browser says, once it said something. */
    recheck(control: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement): void {
        // say nothing new before
        if (untrack(this.refusal) !== "") {
            this.#setRefusal(control.validity.valid ? "" : control.validationMessage);
        }
    }

    /** Describe the control by an element's id while the element is on screen. */
    describe(id: string, isShown: Accessor<boolean>): void {
        this.#descriptions.add({ id, isShown });
    }

    /** Label a group of controls by the field's label until the group unmounts. */
    group(): void {
        this.#groups.add(this.id);
    }

    /** Return the attributes that connect a group of controls to its field. */
    groupAttributes(): FieldGroupAttributes {
        const descriptions = this.descriptions();

        return {
            id: this.id,
            "aria-labelledby": this.labelId,
            "aria-describedby": descriptions.length > 0 ? descriptions.join(" ") : undefined,
            "aria-invalid": this.isInvalid() ? "true" : undefined,
            "aria-disabled": this.isDisabled() ? "true" : undefined,
        };
    }

    /** Return the attributes that connect a control element to its field. */
    attributes(): FieldControlAttributes {
        const descriptions = this.descriptions();

        return {
            id: this.id,
            name: this.value?.path.join("."),
            "aria-describedby": descriptions.length > 0 ? descriptions.join(" ") : undefined,
            "aria-invalid": this.isInvalid() ? "true" : undefined,
            disabled: this.isDisabled() ? true : undefined,
        };
    }
}

/** A description or an error of a field's control. */
interface FieldDescriptionState {
    /** The id of its element. */
    readonly id: string;
    /** Whether its element is on screen. */
    readonly isShown: Accessor<boolean>;
}

/** The attributes that connect a control element to its field. */
export interface FieldControlAttributes {
    /** The id the field's label points at. */
    readonly id: string;
    /** The name a browser fills it in by, its path in the form. */
    readonly name: string | undefined;
    /** The ids of the field's descriptions and errors on screen. */
    readonly "aria-describedby": string | undefined;
    /** Whether the field's value is invalid. */
    readonly "aria-invalid": "true" | undefined;
    /** Whether the field is disabled. */
    readonly disabled: true | undefined;
}

/** The attributes that connect a group of controls to its field. */
export interface FieldGroupAttributes {
    /** The id the field's label points at. */
    readonly id: string;
    /** The id of the field's label, which names the group. */
    readonly "aria-labelledby": string;
    /** The ids of the field's descriptions and errors on screen. */
    readonly "aria-describedby": string | undefined;
    /** Whether the field's value is invalid. */
    readonly "aria-invalid": "true" | undefined;
    /** Whether the field is disabled. */
    readonly "aria-disabled": "true" | undefined;
}

/** Read the control of the nearest field, null outside a field. */
export function useFieldControl(): FieldControl | null {
    return useContext(FieldContext);
}

/** How a control reads and writes its value in its field's form. */
export interface FieldValueOptions {
    /** The property with the control's value. */
    readonly value: string;
    /** The property that handles it changing. */
    readonly change: string;
    /** Read the value the form has as the control's. */
    readonly read: (value: unknown) => unknown;
    /** Read the value a change gives, its first argument by default. */
    readonly take?: (change: unknown, current: unknown) => unknown;
    /** Whether a change settles the value, as a choice does and a letter does not. */
    readonly settles?: boolean;
}

/** Edit the value of the nearest field's form, unless the caller gives one. */
export function useFieldValue<Properties extends object>(
    properties: Properties,
    options: FieldValueOptions,
): Properties {
    // leave a control outside a form, and one its caller controls
    const value = useFieldControl()?.value;
    const { take, read, change } = options;
    if (value === undefined || untrack(() => options.value in properties)) {
        return properties;
    }

    // set the form's value, then tell the caller
    const handle = (given: unknown, ...rest: readonly unknown[]): void => {
        // take what the change gives
        const current = untrack(() => value.value());
        value.set(take === undefined ? given : take(given, current));
        if (options.settles === true) {
            value.settle();
        }
        const told: unknown = Reflect.get(properties, change);
        if (typeof told === "function") {
            Reflect.apply(told, undefined, [given, ...rest]);
        }
    };
    const bound = Object.defineProperties(
        {},
        {
            [options.value]: { enumerable: true, get: () => read(value.value()) },
            [change]: { enumerable: true, value: handle },
        },
    );

    return merge(properties, bound);
}

/** Read text, empty for none. */
export function readText(value: unknown): string {
    return typeof value === "string" ? value : typeof value === "number" ? String(value) : "";
}

/** Read the text of an input as it changes. */
export function takeText(change: unknown): string {
    return change instanceof Event &&
        (change.currentTarget instanceof HTMLInputElement ||
            change.currentTarget instanceof HTMLTextAreaElement)
        ? change.currentTarget.value
        : "";
}

/** Read one choice, none for other values. */
export function readChoice(value: unknown): string | undefined {
    return typeof value === "string" ? value : undefined;
}

/** Read one choice or several, none for other values. */
export function readChoices(value: unknown): string | readonly string[] | undefined {
    return Array.isArray(value)
        ? value.filter((each) => typeof each === "string")
        : readChoice(value);
}
