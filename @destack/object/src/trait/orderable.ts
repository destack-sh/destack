import { type Column, index, type TableConstraint } from "@destack/db";
import { present } from "@destack/schema";
import { field } from "../field/field.ts";
import { FractionalIndex } from "../field/fractional-index.ts";
import type { ObjectDefinition, ObjectType } from "../object/object.ts";
import type { Call } from "../method/call.ts";
import type { Erasure, Trait } from "./trait.ts";

/** How a type orders its objects: among siblings of one parent, or of one field value. */
export type OrderableDefinition = true | { readonly within: string };

/** The ordering of an object type: the columns identifying each object's siblings. */
export interface Ordering {
    /** The properties siblings share beside scope: the parent's, a given field's, or none. */
    readonly within: readonly string[];
}

/** The field ordering adds: each object's place among its siblings. */
export type OrderedFieldsOf<Declared> = Declared extends OrderableDefinition
    ? {
          /** The object's place among its siblings. */
          readonly position: ReturnType<typeof Orderable.positionField>;
      }
    : {};

/** Objects ordered among their siblings by a fractional index, new ones placed after the last. */
export const Orderable = {
    /** Add the position field an ordered definition keeps, refusing a field it declares itself. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, "fields"> {
        // keep a definition ordering nothing
        if (definition.orderable === undefined) {
            return definition;
        }

        // add the position field
        const declared = definition.fields ?? {};
        if (Object.hasOwn(declared, "position")) {
            throw new TypeError(
                `object ${definition.name} cannot declare field position: its ordering adds it`,
            );
        }

        return { ...definition, fields: { ...declared, position: Orderable.positionField() } };
    },

    /** Resolve a definition's ordering to the sibling properties, absent if unordered. */
    of(
        definition: Pick<ObjectDefinition, "name" | "orderable" | "nested" | "fields">,
    ): Ordering | undefined {
        // order nothing without the trait
        const orderable = definition.orderable;
        if (orderable === undefined) {
            return undefined;
        }

        // order within the given field, the parent, or the whole scope
        if (orderable !== true) {
            if (!Object.hasOwn(definition.fields ?? {}, orderable.within)) {
                throw new TypeError(
                    `object ${definition.name} orders within ${orderable.within}, but does not declare it`,
                );
            }

            return { within: [orderable.within] };
        } else if (definition.nested === undefined) {
            return { within: [] };
        } else if (definition.nested.in === "any") {
            return { within: ["parentType", "parentId"] };
        } else {
            return { within: ["parentId"] };
        }
    },

    /** Place a created object after its last sibling. */
    async next(
        object: ObjectType,
        values: Readonly<Record<string, unknown>>,
        call: Call,
    ): Promise<string> {
        // read the last sibling's position in the object's scope
        const ordering = present(object.ordering, `the ordering of ${object.name}`);
        const siblings = Object.fromEntries(
            ordering.within.map((name) => {
                const value = values[name];

                return [name, typeof value === "string" ? value : { isNull: true }];
            }),
        );
        const scope = present(
            typeof values["scope"] === "string" ? values["scope"] : undefined,
            "the created object's scope",
        );
        const [last] = await call
            .read(object.table)
            .findMany({ where: { ...siblings, scope }, orderBy: { position: "desc" }, limit: 1 });

        return FractionalIndex.place(
            typeof last?.["position"] === "string" ? last["position"] : undefined,
            undefined,
        );
    },

    /** Index the siblings in order within the objects' scope. */
    order(
        options: Ordering,
        table: string,
        columns: Readonly<Record<string, Column>>,
    ): TableConstraint {
        // index the scope, the sibling columns, then the position
        const scope = columns["scope"];
        const position = present(columns["position"], "the position column");
        const within = options.within.map((name) =>
            present(columns[name], `the column of ${name}`),
        );
        const [first, ...rest] = [...(scope === undefined ? [] : [scope]), ...within, position];

        return index(`${table}_order`).on(present(first, "the first ordered column"), ...rest);
    },

    /** The field keeping an object's place among its siblings. */
    positionField() {
        return field.fractionalIndex().default(FractionalIndex.between(undefined, undefined));
    },
};

/** Ordered objects, each sibling group in an index of scope, parent or field, and position. */
export const orderable: Trait<Ordering> = {
    key: "orderable",
    options: (definition) => Orderable.of(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => [Orderable.order(options, table, columns)],
    methods: () => ({}),
};
