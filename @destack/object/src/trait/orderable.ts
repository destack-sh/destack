import { type Column, index, type TableConstraint } from "@destack/db";
import { present } from "@destack/schema";
import { field } from "../field/field.ts";
import { FractionalIndex } from "../field/fractional-index.ts";
import type { ObjectDefinition, ObjectType } from "../object/object.ts";
import { ObjectError } from "../error/error.ts";
import type { MethodContext } from "../method/context.ts";
import type { Erasure, TraitBehavior } from "./trait.ts";
import { callableTrait } from "./trait.ts";

/** The options of ordering objects among their siblings. */
export interface OrderedDefinition {
    /** The field whose value siblings share, the parent of a nested type when absent. */
    readonly within?: string;
    /** The permission moving an object among its siblings, the nesting's when that has one. */
    readonly move?: string;
}

/** How a type orders its objects: among siblings of one parent, or of one field value. */
export type OrderableDefinition = true | OrderedDefinition;

/** Where an object goes among its siblings: last, or after one, first for null. */
export type Placement = "last" | { readonly after: string | null };

/** The ordering of an object type: the columns identifying each object's siblings. */
export interface Ordering {
    /** The properties siblings share beside scope: the parent's, a given field's, or none. */
    readonly within: readonly string[];
}

/** The field ordering adds: each object's place among its siblings. */
export type OrderedFieldsOf<Definition> = Definition extends OrderableDefinition
    ? {
          /** The object's place among its siblings. */
          readonly position: ReturnType<typeof Orderable.positionField>;
      }
    : {};

/** Objects ordered among their siblings by a fractional index, new ones placed after the last. */
export const Orderable = {
    /** Add the position field an ordered definition has, refusing a field it declares itself. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, "fields"> {
        // leave a definition ordering nothing
        if (definition.orderable === undefined) {
            return definition;
        }

        // add the position field
        const declared = definition.fields ?? {};
        if (Object.hasOwn(declared, "position")) {
            throw new TypeError(
                `object '${definition.name}' cannot declare field position: its ordering adds it`,
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
        if (orderable !== true && orderable.within !== undefined) {
            if (!Object.hasOwn(definition.fields ?? {}, orderable.within)) {
                throw new TypeError(
                    `object '${definition.name}' orders within '${orderable.within}', but does not declare it`,
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

    /** Place an object with some values among its siblings there. */
    async place(
        object: ObjectType,
        values: Readonly<Record<string, unknown>>,
        placement: Placement,
        context: MethodContext,
    ): Promise<string> {
        // read the siblings in the object's scope, leaving out the object
        const ordering = present(object.ordering, `the ordering of ${object.name}`);
        const shared = Object.fromEntries(
            ordering.within.map((name) => {
                const value = values[name];

                return [name, typeof value === "string" ? value : { isNull: true }];
            }),
        );
        const scope = present(
            typeof values["scope"] === "string" ? values["scope"] : undefined,
            "the ordered object's scope",
        );
        const id = typeof values["id"] === "string" ? { id: { ne: values["id"] } } : {};
        const siblings = { ...shared, ...id, scope };
        const reader = context.read(object.table);
        const edge = async (direction: "asc" | "desc", where: object) => {
            const [row] = await reader.findMany({
                where: { ...siblings, ...where },
                orderBy: { position: direction },
                limit: 1,
            });

            return row === undefined ? undefined : FractionalIndex.parse(row["position"]);
        };

        // find the positions before and after the place
        const { random } = context.environment;
        if (placement === "last") {
            return FractionalIndex.place(await edge("desc", {}), undefined, random);
        } else if (placement.after === null) {
            return FractionalIndex.place(undefined, await edge("asc", {}), random);
        }
        const anchor = await reader.findFirst({
            where: { AND: [{ ...siblings }, { id: placement.after }] },
        });
        if (anchor === undefined) {
            throw ObjectError.of(
                "MOVE_INVALID",
                `move of ${object.name} is invalid: '${placement.after}' is not its sibling`,
            );
        }
        const position = FractionalIndex.parse(anchor["position"]);

        return FractionalIndex.place(
            position,
            await edge("asc", { position: { gt: position } }),
            random,
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

    /** The field storing an object's place among its siblings. */
    positionField() {
        return field.fractionalIndex().default(FractionalIndex.between(undefined, undefined));
    },
};

/** Ordered objects, each sibling group in an index of scope, parent or field, and position. */
const orderableTrait: TraitBehavior<Ordering> = {
    key: "orderable",
    options: (definition) => Orderable.of(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => [Orderable.order(options, table, columns)],
    methods: () => ({}),
};

/** Order objects among their siblings by position. */
export const orderable = callableTrait("orderable", orderableTrait, true);
