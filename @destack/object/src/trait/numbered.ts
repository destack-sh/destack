import { type Snapshot, defineTable, identifier, integer, text, unique } from "@destack/db";
import { present, schema } from "@destack/schema";
import { field } from "../field/field.ts";
import { ObjectError } from "../error/error.ts";
import type { CallLocation, MethodContext } from "../method/context.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";
import type { ObjectDefinition, ObjectType } from "../object/object.ts";
import type { Erasure, TraitBehavior } from "./trait.ts";
import { callableTrait, derivePermissions } from "./trait.ts";

/** The permission whose holders set numbers themselves, as an import keeping its numbers. */
const ASSIGN = "assign";

/** A number an object had before it moved: the value it counted within, and the number. */
const FormerNumber = schema.object({
    /** The value the number counted within. */
    within: schema.string(),
    /** The number. */
    number: schema.int().min(1),
});

/** How a type numbers its objects, when, and how their identifiers read. */
export type NumberedDefinition =
    | true
    | {
          /** The column whose values each start a count, the whole scope when absent. */
          readonly within?: string;
          /** The first number of each count, 1 when absent. */
          readonly start?: number;
          /** The method issuing the number, such as a finalize transition; creation when absent. */
          readonly at?: string;
          /** How identifiers read: from the prefix and key at each read, or fixed when issued. */
          readonly identifier?: "live" | "frozen";
          /** The literal text an identifier starts with, such as "#" or "INV-". */
          readonly prefix?: string;
      };

/** The numbering of an object type, its options resolved. */
export interface Numbering {
    /** The column numbers count within, "scope" when the type counts across its scope. */
    readonly within: string;
    /** The first number of each count. */
    readonly start: number;
    /** The method issuing the number, absent when creation issues it. */
    readonly at: string | undefined;
    /** How identifiers read, absent for a type showing numbers alone. */
    readonly identifier: "live" | "frozen" | undefined;
    /** The literal text identifiers start with, absent for none. */
    readonly prefix: string | undefined;
}

/** The fields numbering adds: the number, and the identifier or former numbers it keeps. */
export type NumberedFieldsOf<Definition> = Definition extends NumberedDefinition
    ? {
          /** The object's number, null until its store issues it. */
          readonly number: ReturnType<typeof Numbered.numberField>;
      } & (Definition extends { readonly identifier: "frozen" }
          ? {
                /** The identifier as issued, never read again from the prefix and key. */
                readonly identifier: ReturnType<typeof Numbered.identifierField>;
            }
          : Definition extends { readonly identifier: "live"; readonly within: string }
            ? {
                  /** The numbers the object had before it moved, which old identifiers find. */
                  readonly formerNumbers: ReturnType<typeof Numbered.formerField>;
              }
            : {})
    : {};

/** The last number each type issued within each value it counts within, never reused. */
export const counter = defineTable("counter", {
    /** The scope with the numbered objects. */
    scope: text("scope").primaryKey(),
    /** The package declaring the numbered type. */
    packageId: identifier("package_id", "package").primaryKey(),
    /** The numbered type. */
    type: text("type").primaryKey(),
    /** The value numbers count within: a parent's or a field's, or the scope itself. */
    within: text("within").primaryKey(),
    /** The last number issued. */
    last: integer("last").notNull(),
});

/** Objects numbered within a scope or a column's value, each number issued once. */
export const Numbered = {
    /** Add the fields a numbered definition has, refusing fields it declares itself. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, "fields"> {
        // leave a definition numbering nothing
        const numbered = definition.numbered;
        if (numbered === undefined) {
            return definition;
        }

        // add the number, and the frozen identifier or the former numbers a live one finds
        const options = numbered === true ? {} : numbered;
        const added = {
            number: Numbered.numberField(),
            ...(options.identifier === "frozen" ? { identifier: Numbered.identifierField() } : {}),
            ...(options.identifier === "live" && options.within !== undefined
                ? { formerNumbers: Numbered.formerField() }
                : {}),
        };

        // refuse a declared field of the same name
        const declared = definition.fields ?? {};
        const taken = Object.keys(added).find((name) => Object.hasOwn(declared, name));
        if (taken !== undefined) {
            throw new TypeError(
                `object '${definition.name}' cannot declare field '${taken}': its numbering adds it`,
            );
        }

        return { ...definition, fields: { ...declared, ...added } };
    },

    /** The field storing an object's number, set once, by holders of the assign permission only. */
    numberField() {
        return field.integer().optional().immutable().guardWrite(ASSIGN);
    },

    /** The field storing a frozen identifier, set once, by holders of the assign permission. */
    identifierField() {
        return field.string().optional().immutable().guardWrite(ASSIGN);
    },

    /** The field storing the numbers an object had before it moved. */
    formerField() {
        return field.json(schema.array(FormerNumber)).optional().immutable().guardWrite(ASSIGN);
    },

    /** Resolve a definition's numbering, refusing a column it does not have or a bad start. */
    of(
        definition: Pick<ObjectDefinition, "name" | "numbered" | "nested" | "fields">,
    ): Numbering | undefined {
        // number nothing without the trait
        const numbered = definition.numbered;
        if (numbered === undefined) {
            return undefined;
        }
        const options = numbered === true ? {} : numbered;
        const within = options.within ?? "scope";

        // require the required parent of a nested type, or a required field
        const { nested } = definition;
        const declared = definition.fields?.[within];
        const isParent = within === "parentId" && nested !== undefined;
        if (within !== "scope" && !isParent && declared === undefined) {
            throw new TypeError(
                `object '${definition.name}' numbers within '${within}', but does not declare it`,
            );
        } else if (
            within !== "scope" &&
            (isParent ? nested?.optional === true : declared?.required !== true)
        ) {
            throw new TypeError(
                `object '${definition.name}' numbers within '${within}', which every object must have`,
            );
        }

        // require a whole first number of at least 1
        const start = options.start ?? 1;
        if (!Number.isInteger(start) || start < 1) {
            throw new TypeError(`object '${definition.name}' starts its numbers at ${start}`);
        }

        return {
            within,
            start,
            at: options.at,
            identifier: options.identifier,
            prefix: options.prefix,
        };
    },

    /** Read the number a store issues next within a row's value: one past its counter's last. */
    async next(
        object: ObjectType,
        values: Readonly<Record<string, unknown>>,
        context: MethodContext,
    ): Promise<number> {
        // count on from the counter, which no deletion lowers, starting at the first number
        const numbering = Numbered.numbering(object);
        const key = {
            ...Numbered.counterKey(object, values),
            within: Numbered.within(object, values),
        };
        const [issued] = await context.read(counter).findMany({ where: key });

        return issued === undefined ? numbering.start : issued.last + 1;
    },

    /** Read the columns a creation writes: none when assigned or issued later, else its number. */
    async created(
        object: ObjectType,
        values: Readonly<Record<string, unknown>>,
        context: MethodContext,
        location: Pick<CallLocation, "kind">,
    ): Promise<Readonly<Record<string, unknown>>> {
        // keep an assigned number, and leave one a later method issues unset
        const assigned = values["number"];
        if (assigned !== undefined && assigned !== null) {
            return {};
        } else if (Numbered.numbering(object).at !== undefined) {
            return { number: null };
        }

        return Numbered.take(object, values, context, location);
    },

    /** Take the next number, as the columns it writes: the number and any frozen identifier. */
    async take(
        object: ObjectType,
        values: Readonly<Record<string, unknown>>,
        context: MethodContext,
        location: Pick<CallLocation, "kind">,
    ): Promise<Readonly<Record<string, unknown>>> {
        // leave a prediction's number to the store, and refuse a sandbox, which cannot read its counter
        const numbering = Numbered.numbering(object);
        if (location.kind === "client") {
            return { number: null };
        } else if (location.kind === "sandbox") {
            throw new TypeError(`${object.name} numbers only in its store, not in a hook`);
        }

        // take the next number, fixing the identifier it reads as now when frozen
        const number = await Numbered.next(object, values, context);
        if (numbering.identifier !== "frozen") {
            return { number };
        }
        const key = await Numbered.withinKey(object, values, context);

        return { number, identifier: Numbered.format(numbering, key, number) };
    },

    /** Read the columns a target writes once moved or at the method issuing its number. */
    async changed(
        object: ObjectType,
        target: Readonly<Record<string, unknown>>,
        changes: Readonly<Record<string, unknown>>,
        context: MethodContext,
        location: Pick<CallLocation, "kind">,
    ): Promise<Readonly<Record<string, unknown>>> {
        // issue a number at the method issuing it, once
        const numbering = Numbered.numbering(object);
        const values = { ...target, ...changes };
        if (context.name === numbering.at && target["number"] === null) {
            return Numbered.take(object, values, context, location);
        }

        // keep the number of a target staying within its value
        const moved = changes[numbering.within];
        if (
            moved === undefined ||
            moved === target[numbering.within] ||
            target["number"] === null
        ) {
            return {};
        }

        // refuse moving a frozen identifier, and renumber a live one, keeping its former number
        if (numbering.identifier === "frozen") {
            throw ObjectError.of(
                "OBJECT_BLOCKED",
                `cannot move ${object.name}: its identifier is frozen`,
            );
        }
        const former = schema.array(FormerNumber).parse(target["formerNumbers"] ?? []);
        const was = { within: Numbered.within(object, target), number: target["number"] };

        return {
            ...(await Numbered.take(object, values, context, location)),
            ...(numbering.identifier === "live"
                ? { formerNumbers: [...former, FormerNumber.parse(was)] }
                : {}),
        };
    },

    /** Raise the counter to a number a store issued, so no later object takes it again. */
    async raise(
        object: ObjectType,
        row: Readonly<Record<string, unknown>>,
        context: MethodContext,
        execution: StoreExecution,
    ): Promise<void> {
        // record nothing for an object still waiting for its number
        if (row["number"] === null || row["number"] === undefined) {
            return;
        }

        // raise the counter to the issued number, which an assigned lower number leaves as it is
        const key = { ...Numbered.counterKey(object, row), within: Numbered.within(object, row) };
        const [before] = await context.read(counter).findMany({ where: key });
        const issued = schema.number().parse(row["number"]);
        if (before !== undefined && before.last >= issued) {
            return;
        }
        await Execution.write(execution, context, {
            table: counter,
            before: before ?? null,
            after: { ...key, last: issued },
        });
    },

    /** Read the number a store issued an object, which only a prediction lacks. */
    number(row: { readonly number: number | null }): number {
        return present(row.number, "the number its store issued");
    },

    /** Read an object's identifier, such as TRK-214, absent while it waits for its number. */
    async identifier(
        object: ObjectType,
        row: Readonly<Record<string, unknown>>,
        reader: Pick<Snapshot, "read">,
    ): Promise<string | undefined> {
        // read a frozen identifier as issued, and none before the number
        const numbering = Numbered.numbering(object);
        if (numbering.identifier === undefined) {
            throw new TypeError(`object ${object.name} has no identifiers`);
        } else if (numbering.identifier === "frozen") {
            return schema.string().nullable().parse(row["identifier"]) ?? undefined;
        } else if (row["number"] === null) {
            return undefined;
        }

        // read a live one from the prefix and the current key
        const key = await Numbered.withinKey(object, row, reader);

        return Numbered.format(numbering, key, schema.number().parse(row["number"]));
    },

    /** Find the object an identifier names in a scope, by its number now or before it moved. */
    async find(
        object: ObjectType,
        scope: string,
        named: string,
        reader: Pick<Snapshot, "read">,
    ): Promise<Readonly<Record<string, unknown>> | undefined> {
        // find a frozen identifier as issued
        const numbering = Numbered.numbering(object);
        if (numbering.identifier === "frozen") {
            return reader.read(object.table).findFirst({ where: { scope, identifier: named } });
        }

        // split the prefix, the key and the number, refusing anything else
        const prefix = numbering.prefix ?? "";
        const rest = named.startsWith(prefix) ? named.slice(prefix.length) : undefined;
        const withinType = Numbered.withinType(object);
        const separator = withinType === undefined ? -1 : (rest?.lastIndexOf("-") ?? -1);
        const number = Number(rest?.slice(separator + 1));
        if (rest === undefined || !Number.isInteger(number) || number < 1) {
            return undefined;
        } else if (withinType !== undefined && separator < 1) {
            return undefined;
        } else if (withinType === undefined) {
            return reader.read(object.table).findFirst({ where: { scope, number } });
        }

        // find the value the key addresses, then the object with the number now or before
        const within = await withinType.holder(reader, scope, rest.slice(0, separator));
        if (within === undefined) {
            return undefined;
        }
        const current = await reader
            .read(object.table)
            .findFirst({ where: { scope, [numbering.within]: within, number } });
        if (current !== undefined) {
            return current;
        }
        const moved = await reader
            .read(object.table)
            .findMany({ where: { scope, formerNumbers: { isNotNull: true } } });

        return moved.find((row) =>
            schema
                .array(FormerNumber)
                .parse(row["formerNumbers"])
                .some((each) => each.within === within && each.number === number),
        );
    },

    /** Format an identifier: the prefix, the key of the value counted within, and the number. */
    format(numbering: Numbering, key: string | undefined, number: number): string {
        return `${numbering.prefix ?? ""}${key === undefined ? "" : `${key}-`}${number}`;
    },

    /** Read the type whose key names the value numbers count within, absent for the scope. */
    withinType(object: ObjectType): ObjectType | undefined {
        // read none across the scope, the parent's type, or the referenced type
        const numbering = Numbered.numbering(object);
        const parent = object.parent?.object;
        if (numbering.within === "scope") {
            return undefined;
        } else if (numbering.within === "parentId") {
            return parent === "any" ? undefined : parent;
        }

        return object.fields[numbering.within]?.target?.();
    },

    /** Read the key of the value a row counts within, absent across the scope. */
    async withinKey(
        object: ObjectType,
        values: Readonly<Record<string, unknown>>,
        reader: Pick<Snapshot, "read"> | Pick<MethodContext, "read">,
    ): Promise<string | undefined> {
        // read none across the scope, or without identifiers
        const withinType = Numbered.withinType(object);
        if (withinType === undefined || Numbered.numbering(object).identifier === undefined) {
            return undefined;
        }

        // read the key of the value's object
        const keyed = present(withinType.keyed, `the key of ${withinType.name}`);
        const id = Numbered.within(object, values);
        const found = await reader.read(withinType.table).findFirst({ where: { id } });

        return schema.string().parse(present(found, `${withinType.name} '${id}'`)[keyed]);
    },

    /** Read a numbered type's numbering, which every caller of these verbs has. */
    numbering(object: ObjectType): Numbering {
        return present(object.numbering, `the numbering of ${object.name}`);
    },

    /** Read the value a row's number counts within. */
    within(object: ObjectType, values: Readonly<Record<string, unknown>>): string {
        return schema.string().parse(values[Numbered.numbering(object).within]);
    },

    /** Read the counter's key of a row's type in its scope. */
    counterKey(object: ObjectType, values: Readonly<Record<string, unknown>>) {
        return { scope: Numbered.scope(values), packageId: object.package.id, type: object.name };
    },

    /** Read a row's scope. */
    scope(values: Readonly<Record<string, unknown>>): string {
        return schema.string().parse(values["scope"]);
    },
};

/** Numbered objects, each number unique within its scope or column value. */
const numberedTrait: TraitBehavior<Numbering> = {
    key: "numbered",
    storages: ["durable"],
    options: (definition) => Numbered.of(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => [
        unique(`${table}_number`).on(
            present(columns[options.within], `the column of ${options.within}`),
            present(columns["number"], "the number column"),
        ),
    ],
    // let only roles grant setting a number, as an import keeping its numbers needs
    policy: (_options, _object, permissions) => derivePermissions(permissions, [ASSIGN]),
    methods: () => ({}),
    validate: (options, object) => {
        // require a mutating method issuing the number, when another than creation does
        const issuing = options.at === undefined ? undefined : object.methods[options.at];
        if (options.at !== undefined && issuing?.mutates !== true) {
            throw new TypeError(
                `object ${object.name} issues numbers at '${options.at}', which is no method changing it`,
            );
        }

        // require identifiers counting within a value to read that value's key, so each names one object
        const withinType = Numbered.withinType(object);
        if (
            options.identifier !== undefined &&
            options.within !== "scope" &&
            withinType?.keyed === undefined
        ) {
            throw new TypeError(
                `object ${object.name} has identifiers, but the '${options.within}' it counts within has no key`,
            );
        }
    },
};

/** Number objects within their scope or a column's value, and name them by identifiers. */
export const numbered = callableTrait("numbered", numberedTrait, true);
