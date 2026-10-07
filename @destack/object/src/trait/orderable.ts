import {
    and,
    type Column,
    desc,
    eq,
    index,
    isNull,
    type DatabaseConnection,
    type TableConstraint,
    TABLE,
} from "@destack/db";
import { present } from "@destack/schema";
import { field } from "../field/field.ts";
import { FractionalIndex } from "../field/fractional-index.ts";
import type { ObjectDefinition, ObjectType } from "../object/object.ts";
import type { Erasure, Trait } from "./trait.ts";

/** How an object type orders its objects: among the siblings under one parent, or among those holding one value of a field. */
export type OrderableDefinition = true | { readonly within: string };

/** The ordering of an object type: the columns naming each object's siblings. */
export interface Ordering {
    /** The properties siblings share beside their scope: the parent's for a nested type, the named field's, or none. */
    readonly within: readonly string[];
}

/** The field ordering adds: each object's place among its siblings. */
export type OrderedFieldsOf<Declared> = Declared extends OrderableDefinition
    ? {
          /** The object's place among its siblings. */
          readonly position: ReturnType<typeof positionField>;
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
                `object ${definition.name} declares field position, which its ordering adds`,
            );
        }

        return { ...definition, fields: { ...declared, position: positionField() } };
    },

    /** Resolve a definition's ordering to the properties naming siblings, absent for one ordering nothing. */
    of(
        definition: Pick<ObjectDefinition, "name" | "orderable" | "nested" | "fields">,
    ): Ordering | undefined {
        // order nothing without the trait
        const orderable = definition.orderable;
        if (orderable === undefined) {
            return undefined;
        }

        // order within the named field, the parent, or the whole scope
        if (orderable !== true) {
            if (!Object.hasOwn(definition.fields ?? {}, orderable.within)) {
                throw new TypeError(
                    `object ${definition.name} orders within ${orderable.within}, which it does not declare`,
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
        database: DatabaseConnection,
    ): Promise<string> {
        // read the last sibling's position in the object's scope
        const table = object.table;
        const ordering = present(object.ordering, `the ordering of ${object.name}`);
        const siblings = ordering.within.map((name) => {
            const column = table[TABLE].column(name);
            const value = values[name];

            return typeof value === "string" ? eq(column, value) : isNull(column);
        });
        const scope = present(
            typeof values["scope"] === "string" ? values["scope"] : undefined,
            "the created object's scope",
        );
        const [last] = await database
            .select({ position: table[TABLE].column("position") })
            .from(table)
            .where(and(eq(table[TABLE].column("scope"), scope), ...siblings))
            .orderBy(desc(table[TABLE].column("position")))
            .limit(1);

        return FractionalIndex.place(
            typeof last?.position === "string" ? last.position : undefined,
            undefined,
        );
    },
};

/** Ordered objects, each sibling group kept in an index of the scope, the parent or the field, and the position. */
export const orderable: Trait<Ordering> = {
    key: "orderable",
    options: (definition) => Orderable.of(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => [order(options, table, columns)],
    methods: () => ({}),
};

/** Index the siblings in order within the objects' scope. */
function order(
    options: Ordering,
    table: string,
    columns: Readonly<Record<string, Column>>,
): TableConstraint {
    // index the scope, the sibling columns, then the position
    const scope = columns["scope"];
    const position = present(columns["position"], "the position column");
    const within = options.within.map((name) => present(columns[name], `the column of ${name}`));
    const [first, ...rest] = [...(scope === undefined ? [] : [scope]), ...within, position];

    return index(`${table}_order`).on(present(first, "the first ordered column"), ...rest);
}

/** The field keeping an object's place among its siblings. */
function positionField() {
    return field.fractionalIndex().default(FractionalIndex.between(undefined, undefined));
}
