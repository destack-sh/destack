import type { Direction, OrderBy } from "@destack/db";
import type { IconBodies } from "@destack/icon";
import { type Localization, type Message, t } from "@destack/locale";
import type { ActionValue } from "@destack/object";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createMemo,
    untrack,
    useContext,
} from "@destack/view";

/** A field a person sorts by. */
export interface SortFieldDefinition {
    /** What it is called. */
    readonly label: string;
    /** What it has, which words its directions, such as from A to Z. */
    readonly type?: Pick<ActionValue, "kind"> | undefined;
    /** Its icon, its kind's by default. */
    readonly icon?: IconBodies | undefined;
}

/** The fields a person sorts by, by the names orders have. */
export type SortFields = Readonly<Record<string, SortFieldDefinition>>;

/** A field rows sort by, and its direction. */
export interface SortKey {
    /** The field. */
    readonly field: string;
    /** The direction. */
    readonly direction: Direction;
}

/** The order a sort edits. */
export interface SortOptions {
    /** The fields a person sorts by. */
    readonly fields: SortFields;
    /** The order, which makes the sort controlled, as a query takes it. */
    readonly value?: OrderBy | undefined;
    /** The order an uncontrolled sort starts from. */
    readonly defaultValue?: OrderBy | undefined;
    /** Handle the order changing. */
    readonly onValueChange?: (value: OrderBy) => void;
}

/** What a direction is called, ascending first. */
type DirectionWords = readonly [ascending: Message, descending: Message];

/** What the directions of a kind of value are called. */
const WORDS: Readonly<Record<string, DirectionWords>> = {
    text: [t`A to Z`, t`Z to A`],
    number: [t.context("option")`Lowest first`, t.context("option")`Highest first`],
    time: [t.context("option")`Oldest first`, t.context("option")`Newest first`],
};

/** What the directions of any other value are called. */
const PLAIN: DirectionWords = [t.context("option")`Ascending`, t.context("option")`Descending`];

/** The keys of a sort, which its parts edit. */
export class SortControl {
    /** The fields rows sort by, the first deciding first. */
    readonly keys: Accessor<readonly SortKey[]>;
    /** The options. */
    readonly #options: SortOptions;
    /** Replace the order. */
    readonly #give: (value: OrderBy) => void;

    /** Edit an order. */
    constructor(options: SortOptions) {
        // follow the controlled order or the sort's own
        const [value, setValue] = createControllableSignal<OrderBy>({
            isControlled: () => "value" in options,
            value: () => options.value ?? {},
            defaultValue: untrack(() => options.defaultValue) ?? {},
            onChange: (next) => options.onValueChange?.(next),
        });
        this.#options = options;
        this.#give = setValue;
        this.keys = createMemo(() =>
            Object.entries(value()).flatMap(([field, direction]) =>
                direction === undefined ? [] : { field, direction },
            ),
        );
    }

    /** Read the fields a person sorts by. */
    fields(): SortFields {
        return this.#options.fields;
    }

    /** Read a field, undefined for one a person does not sort by. */
    field(name: string): SortFieldDefinition | undefined {
        return this.#options.fields[name];
    }

    /** Read the direction a field sorts in, undefined where it does not. */
    direction(name: string): Direction | undefined {
        return this.keys().find(({ field }) => field === name)?.direction;
    }

    /** List the fields rows do not sort by. */
    others(): readonly string[] {
        return Object.keys(this.fields()).filter((name) => this.direction(name) === undefined);
    }

    /** Say what a direction of a field is called. */
    words(name: string, direction: Direction, locale: Localization): string {
        const [ascending, descending] = WORDS[this.field(name)?.type?.kind ?? ""] ?? PLAIN;

        return locale.render(direction === "asc" ? ascending : descending);
    }

    /** Sort by a field in a direction, after the others where it is not among them. */
    set(name: string, direction: Direction = "asc"): void {
        const isKnown = this.direction(name) !== undefined;
        this.#write(
            isKnown
                ? this.keys().map((key) => (key.field === name ? { ...key, direction } : key))
                : [...this.keys(), { field: name, direction }],
        );
    }

    /** Sort by another field in place of one, in its place. */
    replace(name: string, other: string): void {
        this.#write(
            this.keys().map((key) => (key.field === name ? { ...key, field: other } : key)),
        );
    }

    /** Sort by a field no more. */
    remove(name: string): void {
        this.#write(this.keys().filter(({ field }) => field !== name));
    }

    /** Let a field decide before another, last without one. */
    move(name: string, before: string | undefined): void {
        // take the key out, then insert it before the other
        const moved = this.keys().filter(({ field }) => field === name);
        const others = this.keys().filter(({ field }) => field !== name);
        const found = others.findIndex(({ field }) => field === before);
        const at = found === -1 ? others.length : found;
        this.#write([...others.slice(0, at), ...moved, ...others.slice(at)]);
    }

    /** Sort by nothing. */
    clear(): void {
        this.#write([]);
    }

    /** Give the order of some keys. */
    #write(keys: readonly SortKey[]): void {
        this.#give(Object.fromEntries(keys.map(({ field, direction }) => [field, direction])));
    }
}

/** The sort of the nearest root, null outside one. */
export const SortContext = createContext<SortControl | null>(null);

/** Read the sort of the nearest root, refusing a part outside one. */
export function useSort(): SortControl {
    const control = useContext(SortContext);
    if (control === null) {
        throw new TypeError("a part of a sort needs a sort around it");
    }

    return control;
}
