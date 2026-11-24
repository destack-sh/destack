import { type Localization, type NumberFormat, t } from "@destack/locale";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createSignal,
    type Setter,
    untrack,
    useContext,
} from "@destack/view";
import type { FieldControl } from "../field/control.ts";

/** The most digits after a decimal separator that Intl writes. */
const MOST_DIGITS = 20;

/** The user agents of iPhones and iPads. */
const APPLE_TOUCH = /iPad|iPhone|iPod/u;

/** How far a number steps: a step, a fine one or a coarse one. */
export type NumberFieldStep = "step" | "small" | "large";

/** The number of a field, its limits and how it steps. */
export interface NumberFieldOptions {
    /** The number, which makes the field controlled, undefined while none is entered. */
    readonly value?: number | undefined;
    /** The number the field starts from when uncontrolled. */
    readonly defaultValue?: number | undefined;
    /** Handle the number changing, as a person types it too. */
    readonly onValueChange?: (value: number | undefined) => void;
    /** Handle the number a person settles on, as the field loses the focus. */
    readonly onValueCommitted?: (value: number | undefined) => void;
    /** The lowest number. */
    readonly min?: number | undefined;
    /** The highest number. */
    readonly max?: number | undefined;
    /** How far the arrow keys and the buttons step, 1 by default. */
    readonly step?: number | undefined;
    /** How far they step with Alt, a tenth of a step by default. */
    readonly smallStep?: number | undefined;
    /** How far they step with Shift, ten steps by default. */
    readonly largeStep?: number | undefined;
    /** Whether a step lands on a multiple of the step, counted from the lowest number. */
    readonly snapOnStep?: boolean;
    /** Whether a person may type a number outside the limits. */
    readonly allowOutOfRange?: boolean;
    /** Whether the wheel steps while the field has the focus. */
    readonly allowWheelScrub?: boolean;
    /** How the number is written. */
    readonly format?: NumberFormat | undefined;
    /** Whether the field takes no input. */
    readonly disabled?: boolean | undefined;
    /** Whether the number is shown alone. */
    readonly readOnly?: boolean | undefined;
    /** Whether a form needs a number. */
    readonly required?: boolean | undefined;
    /** The form field name the number submits under. */
    readonly name?: string | undefined;
    /** The id of the form the field belongs to, its ancestor form by default. */
    readonly form?: string | undefined;
    /** What the input shows while no number is entered. */
    readonly placeholder?: string | undefined;
    /** The input's id. */
    readonly id?: string | undefined;
    /** The input's label. */
    readonly "aria-label"?: string | undefined;
    /** The ids of what labels the input. */
    readonly "aria-labelledby"?: string | undefined;
    /** The ids of what describes the input. */
    readonly "aria-describedby"?: string | undefined;
    /** Whether the number is refused. */
    readonly "aria-invalid"?: "true" | "false" | undefined;
}

/** The number of a field and the text a person types, which its parts share. */
export class NumberFieldControl {
    /** The options. */
    readonly options: NumberFieldOptions;
    /** The field around it, null outside one. */
    readonly field: FieldControl | null;
    /** The number, undefined while none is entered. */
    readonly value: Accessor<number | undefined>;
    /** Replace the number and tell the change handler. */
    readonly #setValue: (value: number | undefined) => void;
    /** The text a person types, undefined while the number is written. */
    readonly #text: Accessor<string | undefined>;
    /** Replace the text a person types. */
    readonly #setText: Setter<string | undefined>;
    /** Whether the text a person settled on is no number. */
    readonly #isUnreadable: Accessor<boolean>;
    /** Set whether it is. */
    readonly #setUnreadable: Setter<boolean>;
    /** The language the number is written in. */
    readonly #locale: Localization;

    /** Create the state of a number field. */
    constructor(options: NumberFieldOptions, field: FieldControl | null, locale: Localization) {
        // follow the controlled number or the field's own
        this.options = options;
        this.field = field;
        this.#locale = locale;
        const [value, setValue] = createControllableSignal<number | undefined>({
            isControlled: () => "value" in options,
            value: () => options.value,
            defaultValue: untrack(() => options.defaultValue),
            onChange: (next) => options.onValueChange?.(next),
        });
        this.value = value;
        this.#setValue = setValue;
        const [text, setText] = createSignal<string | undefined>(undefined, { ownedWrite: true });
        this.#text = text;
        const [isUnreadable, setUnreadable] = createSignal(false, { ownedWrite: true });
        this.#isUnreadable = isUnreadable;
        this.#setUnreadable = setUnreadable;
        this.#setText = setText;
    }

    /** Read the text of the field: what a person types, else the number as it is written. */
    shown(): string {
        return this.#text() ?? this.written();
    }

    /** Write the number, empty while none is entered. */
    written(): string {
        const value = this.value();

        return value === undefined ? "" : this.#write(value);
    }

    /** Write a number in the field's format. */
    #write(value: number): string {
        return this.#locale.number(value, {
            ...this.options.format,
            maximumFractionDigits: this.digits(),
        });
    }

    /** Count the digits after the decimal separator the number is written and kept with. */
    digits(): number {
        // keep those the format asks for, else those of its style and of each step
        const { format = {}, step, smallStep } = this.options;
        const isShare = format.style === "percent";
        const { maximumFractionDigits = 0 } = new Intl.NumberFormat("en", format).resolvedOptions();
        const steps = [step ?? 1, smallStep ?? 1].map((each) =>
            decimalsOf(isShare ? precise(each * 100, [each]) : each),
        );
        const isFree = ["decimal", "unit"].includes(format.style ?? "decimal");

        return (
            format.maximumFractionDigits ??
            (isFree ? MOST_DIGITS : Math.max(maximumFractionDigits, ...steps))
        );
    }

    /** Report whether a person may change the number. */
    isEditable(): boolean {
        return !this.isDisabled() && this.options.readOnly !== true;
    }

    /** Report whether the field takes no input. */
    isDisabled(): boolean {
        return this.options.disabled === true || this.field?.isDisabled() === true;
    }

    /** Report whether the number may step up or down. */
    canStep(direction: 1 | -1): boolean {
        // compare it with the limit that way
        const value = this.value();
        const { min, max } = this.options;
        const limit = direction === 1 ? max : min;

        return (
            this.isEditable() &&
            (value === undefined || limit === undefined || (limit - value) * direction > 0)
        );
    }

    /** Take the text a person types, with its number where it is one. */
    type(text: string): void {
        // leave text that is no number yet, such as a lone minus, to what comes after
        this.#setText(text);
        this.#setUnreadable(false);
        const isEmpty = text.trim() === "";
        const value = this.read(text);
        if (isEmpty || value !== undefined) {
            this.#change(value);
        }
    }

    /** Read a number from text in the field's format, undefined for text that is none. */
    read(text: string): number | undefined {
        return this.#locale.parseNumber(text, this.options.format);
    }

    /** Settle on the number typed, within the limits, and write it again. */
    commit(): void {
        // leave a field nobody typed in
        const text = this.#text();
        if (text === undefined) {
            return;
        }

        // leave text that is no number, which the field refuses, to the person
        const typed = this.read(text);
        this.#setUnreadable(typed === undefined && text.trim() !== "");
        if (typed === undefined && text.trim() !== "") {
            this.#change(undefined);
            this.options.onValueCommitted?.(undefined);
        } else {
            const isFree = typed === undefined || this.options.allowOutOfRange === true;
            this.settle(isFree ? typed : this.#clamp(typed));
        }
    }

    /** Read why the number is refused, empty while it is not. */
    refusal(): string {
        // ask for a number in place of other text, once a person is done typing
        if (this.#isUnreadable()) {
            return this.#locale.render(t`Enter a number`);
        }

        // compare it with the limits
        const value = this.#text() === undefined ? this.value() : undefined;
        const { min, max } = this.options;
        if (value !== undefined && min !== undefined && value < min) {
            return this.#locale.render(t`Choose ${this.#write(min)} or more.`);
        } else if (value !== undefined && max !== undefined && value > max) {
            return this.#locale.render(t`Choose ${this.#write(max)} or less.`);
        }

        return "";
    }

    /** Return to the number the field started from, as a form reset does. */
    reset(): void {
        this.#setUnreadable(false);
        this.#setText(undefined);
        this.#change(this.options.defaultValue);
    }

    /** Read the keyboard a phone shows, an iPhone's numeric ones having no minus. */
    keyboard(): "text" | "decimal" | "numeric" {
        // write a minus where the number may be below zero
        const isNegative = (this.options.min ?? -1) < 0;
        const isPhone = typeof navigator !== "undefined" && APPLE_TOUCH.test(navigator.userAgent);

        return isNegative && isPhone ? "text" : this.digits() > 0 ? "decimal" : "numeric";
    }

    /** Step the number up or down. */
    step(direction: 1 | -1, size: NumberFieldStep = "step", times = 1): void {
        // start from nothing at the nearest limit
        if (!this.canStep(direction)) {
            return;
        }
        const current = this.#typed();
        const { min, max, snapOnStep } = this.options;
        const by = this.#stepOf(size) * times;
        const next =
            current === undefined
                ? (min ?? (max !== undefined && max < 0 ? max : 0))
                : snapOnStep === true
                  ? this.#snap(current, by, direction)
                  : current + by * direction;

        // take it within the limits, which settles as the keys, the button or the pointer let go
        this.#take(this.#clamp(precise(next, [current ?? 0, by, min ?? 0])));
    }

    /** Tell that a person settled on the number. */
    finish(): void {
        this.options.onValueCommitted?.(this.value());
    }

    /** Move the number to its lowest or its highest, where it has one. */
    toLimit(limit: "min" | "max"): void {
        // leave a field without it
        const value = this.options[limit];
        if (value !== undefined && this.isEditable()) {
            this.settle(value);
        }
    }

    /** Set the number and tell that a person settled on it. */
    settle(value: number | undefined): void {
        const kept = this.#take(value);
        this.options.onValueCommitted?.(kept);
    }

    /** Set the number, with the digits it is written with. */
    #take(value: number | undefined): number | undefined {
        // leave other text behind
        this.#setUnreadable(false);

        // keep the digits it is written with
        const isShare = this.options.format?.style === "percent";
        const digits = Math.min(this.digits() + (isShare ? 2 : 0), MOST_DIGITS);
        const kept = value === undefined ? undefined : Number(value.toFixed(digits));
        this.#setText(undefined);
        this.#change(kept);

        return kept;
    }

    /** Read the number typed, else the number. */
    #typed(): number | undefined {
        const text = this.#text();

        return text === undefined ? this.value() : (this.read(text) ?? this.value());
    }

    /** Read how far a step goes. */
    #stepOf(size: NumberFieldStep): number {
        const { step = 1, smallStep = step / 10, largeStep = step * 10 } = this.options;

        return size === "small" ? smallStep : size === "large" ? largeStep : step;
    }

    /** Find the next multiple of a step, counted from the lowest number. */
    #snap(value: number, by: number, direction: 1 | -1): number {
        // count the steps from the lowest number
        const base = this.options.min ?? 0;
        const steps = precise((value - base) / by, [value, by, base]);
        const next = direction === 1 ? Math.floor(steps) + 1 : Math.ceil(steps) - 1;

        return base + next * by;
    }

    /** Keep a number within the limits. */
    #clamp(value: number): number {
        const { min = Number.NEGATIVE_INFINITY, max = Number.POSITIVE_INFINITY } = this.options;

        return Math.min(Math.max(value, min), max);
    }

    /** Set the number where it changes. */
    #change(value: number | undefined): void {
        if (value !== this.value()) {
            this.#setValue(value);
        }
    }
}

/** Round a number to the digits of those it comes from, without the noise of binary fractions. */
function precise(value: number, sources: readonly number[]): number {
    const digits = Math.max(...sources.map(decimalsOf));

    return Number(value.toFixed(Math.min(digits + 1, MOST_DIGITS)));
}

/** Count the digits after a number's decimal point. */
function decimalsOf(value: number): number {
    const [digits = "", exponent = "0"] = String(value).split("e");
    const decimals = digits.split(".")[1]?.length ?? 0;

    return Math.max(decimals - Number(exponent), 0);
}

/** The nearest number field, null outside one. */
export const NumberFieldContext = createContext<NumberFieldControl | null>(null);

/** Read the nearest number field, refusing a part outside one. */
export function useNumberField(): NumberFieldControl {
    const control = useContext(NumberFieldContext);
    if (control === null) {
        throw new TypeError("a part of a number field needs a number field around it");
    }

    return control;
}
