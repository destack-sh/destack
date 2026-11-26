import { Condition, type JsonCondition, Predicate } from "@destack/db";
import type { IconBodies } from "@destack/icon";
import { type Localization, t } from "@destack/locale";
import { Choice, type FieldChoice, type NumberInput } from "@destack/object";
import type { JsonValue } from "@destack/schema";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createMemo,
    createSignal,
    untrack,
    useContext,
} from "@destack/view";
import type { ValueKind } from "../value-field/index.ts";
import { FILTER_OPERATORS, FILTER_SPANS, type FilterOperatorDefinition } from "./operator.ts";

/** The names of a condition that combine others. */
const COMBINING: ReadonlySet<string> = new Set(["AND", "OR", "NOT"]);

/** What a field has: its kind, with a number's format or the values of a choice. */
export type FilterType =
    | { readonly kind: Exclude<ValueKind, "number" | "choice"> }
    | (NumberInput & { readonly kind: "number" })
    | { readonly kind: "choice"; readonly choices?: readonly FieldChoice[] };

/** A field a person filters by. */
export interface FilterFieldDefinition {
    /** What it is called. */
    readonly label: string;
    /** What it has. */
    readonly type: FilterType;
    /** Its icon, its kind's by default. */
    readonly icon?: IconBodies | undefined;
    /** The choices a person picks from, those of its kind by default. */
    readonly choices?: readonly Choice[] | undefined;
    /** Find the choices of what a person types, which are then not narrowed here. */
    readonly onSearch?: ((text: string) => void) | undefined;
    /** Count what has a value, shown beside it. */
    readonly count?: ((value: JsonValue) => number | undefined) | undefined;
    /** How it compares, as its kind does by default. */
    readonly operators?: readonly FilterOperatorDefinition[] | undefined;
}

/** The fields a person filters by, by the names conditions have. */
export type FilterFields = Readonly<Record<string, FilterFieldDefinition>>;

/** A condition on one field, as far as a person gave it. */
export interface FilterRule {
    /** What tells it from others. */
    readonly id: number;
    /** What it is. */
    readonly kind: "rule";
    /** The field. */
    readonly field: string;
    /** How it compares. */
    readonly operator: string;
    /** What it compares with. */
    readonly values: readonly JsonValue[];
}

/** Conditions that all hold, or of which any does. */
export interface FilterSet {
    /** What tells it from others. */
    readonly id: number;
    /** What it is. */
    readonly kind: "set";
    /** Whether all hold, or any does. */
    readonly match: "AND" | "OR";
    /** The conditions. */
    readonly items: readonly FilterEntry[];
}

/** A condition no field's comparison writes, kept as it is. */
export interface FilterOther {
    /** What tells it from others. */
    readonly id: number;
    /** What it is. */
    readonly kind: "other";
    /** The condition. */
    readonly where: JsonCondition;
}

/** One condition of a filter. */
export type FilterEntry = FilterRule | FilterSet | FilterOther;

/** The condition a filter edits. */
export interface FilterOptions {
    /** The fields a person filters by. */
    readonly fields: FilterFields;
    /** The condition, which makes the filter controlled, as a query takes it. */
    readonly value?: JsonCondition | undefined;
    /** The condition an uncontrolled filter starts from. */
    readonly defaultValue?: JsonCondition | undefined;
    /** Handle the condition changing, conditions a person has not finished left out. */
    readonly onValueChange?: (value: JsonCondition) => void;
}

/** The conditions of a filter, which its parts edit. */
export class FilterControl {
    /** The conditions, those a person has not finished included. */
    readonly root: Accessor<FilterSet>;
    /** The options. */
    readonly #options: FilterOptions;
    /** The condition. */
    readonly #value: Accessor<JsonCondition>;
    /** Replace the condition. */
    readonly #setValue: (value: JsonCondition) => void;
    /** The conditions as edited, and the condition they were written as. */
    readonly #draft: Accessor<{ readonly root: FilterSet; readonly written: string } | undefined>;
    /** Replace the conditions as edited. */
    readonly #setDraft: (draft: { readonly root: FilterSet; readonly written: string }) => void;
    /** The last entry's id. */
    #last = 0;

    /** Edit a condition. */
    constructor(options: FilterOptions) {
        // follow the controlled condition or the filter's own
        this.#options = options;
        const [value, setValue] = createControllableSignal<JsonCondition>({
            isControlled: () => "value" in options,
            value: () => options.value ?? {},
            defaultValue: untrack(() => options.defaultValue) ?? {},
            onChange: (next) => options.onValueChange?.(next),
        });
        this.#value = value;
        this.#setValue = setValue;
        const [draft, setDraft] = createSignal<
            { readonly root: FilterSet; readonly written: string } | undefined
        >(undefined, { ownedWrite: true });
        this.#draft = draft;
        this.#setDraft = (next) => setDraft(next);

        // keep the edited conditions while they are of the condition, unfinished ones included
        this.root = createMemo(() => {
            const edited = this.#draft();
            const where = this.#value();

            return edited !== undefined && edited.written === JSON.stringify(where)
                ? edited.root
                : this.#read(where);
        });
    }

    /** Read the fields a person filters by. */
    fields(): FilterFields {
        return this.#options.fields;
    }

    /** Read a field, undefined for one a person does not filter by. */
    field(name: string): FilterFieldDefinition | undefined {
        return this.#options.fields[name];
    }

    /** List how a field compares. */
    operators(name: string): readonly FilterOperatorDefinition[] {
        const field = this.field(name);

        return field?.operators ?? FILTER_OPERATORS[field?.type.kind ?? ""] ?? [];
    }

    /** List what a field's values are chosen from, spans for times, none for those typed. */
    choices(name: string, locale: Localization): readonly Choice[] {
        const field = this.field(name);
        if (field?.choices !== undefined) {
            return field.choices;
        }

        return field?.type.kind === "choice"
            ? (field.type.choices ?? []).map((choice) => Choice.of(choice, locale))
            : field?.type.kind === "boolean"
              ? [
                    { value: true, title: locale.render(t.context("option")`yes`) },
                    { value: false, title: locale.render(t.context("option")`no`) },
                ]
              : [];
    }

    /** List the values offered beside a field as a person types: spans of time, or its choices. */
    suggestions(name: string, locale: Localization): readonly Choice[] {
        const isSpan = this.operators(name)[0]?.arity === "span";

        return isSpan
            ? FILTER_SPANS.map((span) => ({ value: span.name, title: locale.render(span.title) }))
            : this.choices(name, locale);
    }

    /** Find how a rule compares. */
    operator(rule: FilterRule): FilterOperatorDefinition | undefined {
        return this.operators(rule.field).find(({ name }) => name === rule.operator);
    }

    /** Add a condition on a field, returning its id. */
    add(field: string, values: readonly JsonValue[] = [], set: number = this.root().id): number {
        // compare as its kind does first
        const [operator] = this.operators(field);
        if (operator === undefined) {
            throw new TypeError(`filter field '${field}' is invalid: it has no operators`);
        }
        const id = this.#next();
        const rule = { id, kind: "rule", field, operator: operator.name, values } as const;
        this.#edit(append(this.root(), set, rule));

        return id;
    }

    /** Add a set of conditions of which any holds, returning its id. */
    addSet(set: number = this.root().id): number {
        const id = this.#next();
        this.#edit(append(this.root(), set, { id, kind: "set", match: "OR", items: [] }));

        return id;
    }

    /** Change a condition's comparison, its values kept where they fit. */
    compare(rule: FilterRule, operator: FilterOperatorDefinition): void {
        const before = this.operator(rule);
        const values =
            before?.arity === operator.arity ||
            (operator.arity === "many" && before?.arity === "one")
                ? rule.values
                : [];
        this.#edit(replace(this.root(), { ...rule, operator: operator.name, values }));
    }

    /** Set what a condition compares with. */
    set(rule: FilterRule, values: readonly JsonValue[]): void {
        this.#edit(replace(this.root(), { ...rule, values }));
    }

    /** Set whether all of a set's conditions hold, or any does. */
    match(set: FilterSet, match: FilterSet["match"]): void {
        this.#edit(replace(this.root(), { ...set, match }));
    }

    /** Remove a condition. */
    remove(id: number): void {
        this.#edit(drop(this.root(), id));
    }

    /** Remove every condition. */
    clear(): void {
        this.#edit({ ...this.root(), items: [] });
    }

    /** Keep the conditions as edited, and give the condition they write. */
    #edit(root: FilterSet): void {
        // give the condition where it changes
        const where: JsonCondition = this.#write(root) ?? {};
        const written = JSON.stringify(where);
        this.#setDraft({ root, written });
        if (written !== JSON.stringify(this.#value())) {
            this.#setValue(where);
        }
    }

    /** Write an entry as a condition, undefined for one a person has not finished. */
    #write(entry: FilterEntry): JsonCondition | undefined {
        // write a field's comparison, and what is kept as it is
        if (entry.kind === "rule") {
            const comparison = this.operator(entry)?.write(entry.values);

            return comparison === undefined ? undefined : { [entry.field]: comparison };
        } else if (entry.kind === "other") {
            return entry.where;
        }

        // write a set's one condition alone, and several together
        const items = entry.items
            .map((item) => this.#write(item))
            .filter((item) => item !== undefined);
        const [first] = items;

        return items.length > 1 ? { [entry.match]: items } : first;
    }

    /** Read a condition as the entries it has. */
    #read(where: JsonCondition): FilterSet {
        const entries = this.#entries(where);
        const [first] = entries;

        return entries.length === 1 && first?.kind === "set"
            ? first
            : { id: this.#next(), kind: "set", match: "AND", items: entries };
    }

    /** Read a condition's entries: its sets, its fields' comparisons, and what else it has. */
    #entries(where: JsonCondition): readonly FilterEntry[] {
        return Object.entries(where).flatMap(([name, entry]): FilterEntry[] => {
            // read the conditions of a set
            if ((name === "AND" || name === "OR") && Array.isArray(entry)) {
                const conditions: readonly unknown[] = entry;
                const items = conditions
                    .filter(Condition.is)
                    .flatMap((condition) => this.#member(condition));

                return [{ id: this.#next(), kind: "set", match: name, items }];
            }

            // read a field's comparison, or its comparisons one by one
            return this.#comparisons(name, entry);
        });
    }

    /** Read one condition of a set, as a set of its own where it has several entries. */
    #member(where: JsonCondition): readonly FilterEntry[] {
        const items = this.#entries(where);

        return items.length > 1 ? [{ id: this.#next(), kind: "set", match: "AND", items }] : items;
    }

    /** Read the comparisons of a field. */
    #comparisons(field: string, entry: JsonCondition[string]): FilterEntry[] {
        // read it whole, as what one operator writes
        const whole = this.#rule(field, entry);
        if (whole !== undefined) {
            return [whole];
        }

        // read each comparison alone, and keep it all as it is where one is not read
        const parts = Condition.is(entry)
            ? Object.entries(entry).map(([name, value]) => this.#rule(field, { [name]: value }))
            : [];
        const rules = parts.filter((rule) => rule !== undefined);

        return rules.length > 0 && rules.length === parts.length
            ? rules
            : [{ id: this.#next(), kind: "other", where: { [field]: entry } }];
    }

    /** Read a comparison as a rule, undefined where no operator of its field writes it. */
    #rule(field: string, entry: JsonCondition[string]): FilterRule | undefined {
        if (isConditions(entry)) {
            return undefined;
        }
        for (const operator of this.operators(field)) {
            const values = operator.read(entry);
            if (values !== undefined) {
                return { id: this.#next(), kind: "rule", field, operator: operator.name, values };
            }
        }

        return undefined;
    }

    /** Count an entry. */
    #next(): number {
        this.#last += 1;

        return this.#last;
    }
}

/** Report whether an entry of a condition is a list of conditions. */
function isConditions(entry: JsonCondition[string]): entry is readonly JsonCondition[] {
    return Array.isArray(entry) && entry.some((each) => typeof each === "object");
}

/** Append an entry to a set. */
function append(set: FilterSet, id: number, entry: FilterEntry): FilterSet {
    return set.id === id
        ? { ...set, items: [...set.items, entry] }
        : {
              ...set,
              items: set.items.map((item) =>
                  item.kind === "set" ? append(item, id, entry) : item,
              ),
          };
}

/** Replace an entry by another with its id. */
function replace(set: FilterSet, entry: FilterEntry): FilterSet {
    if (set.id === entry.id && entry.kind === "set") {
        return entry;
    }

    return {
        ...set,
        items: set.items.map((item) =>
            item.id === entry.id ? entry : item.kind === "set" ? replace(item, entry) : item,
        ),
    };
}

/** Drop an entry. */
function drop(set: FilterSet, id: number): FilterSet {
    return {
        ...set,
        items: set.items
            .filter((item) => item.id !== id)
            .map((item) => (item.kind === "set" ? drop(item, id) : item)),
    };
}

/** List the fields a condition compares. */
function fieldsOf(where: JsonCondition): string[] {
    return Object.entries(where).flatMap(([name, entry]) => {
        // look inside what combines conditions
        if (!COMBINING.has(name)) {
            return [name];
        }
        const conditions: readonly unknown[] = Array.isArray(entry) ? entry : [entry];

        return conditions.filter(Condition.is).flatMap((condition) => fieldsOf(condition));
    });
}

/** Report whether a row meets a condition, as the engine's queries decide it. */
export function matchesFilter(
    where: JsonCondition,
    row: Readonly<Record<string, JsonValue | undefined>>,
    moment: { readonly now: number; readonly timeZone: string },
): boolean {
    // decide it by the fields the row and the condition have
    const fields = new Set([...Object.keys(row), ...fieldsOf(where)]);
    const predicate = Condition.resolve(Condition.at(where, moment), fields);
    const columns = Object.fromEntries([...fields].map((name) => [name, row[name] ?? null]));

    return Predicate.matches(Predicate.compile(predicate), columns);
}

/** The filter of the nearest root, null outside one. */
export const FilterContext = createContext<FilterControl | null>(null);

/** Read the filter of the nearest root, refusing a part outside one. */
export function useFilter(): FilterControl {
    const control = useContext(FilterContext);
    if (control === null) {
        throw new TypeError("a part of a filter needs a filter around it");
    }

    return control;
}
