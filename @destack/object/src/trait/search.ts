import { type Column, index, type TableConstraint } from "@destack/db";
import { defineSchema, present, schema } from "@destack/schema";
import type { ObjectDefinition, ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** How callers find a type's objects. */
const ObjectSearchSchema = defineSchema(
    schema.object({
        /** The fields the space's full-text index covers, the most significant first. */
        searchable: schema.array(schema.string().min(1)),
        /** The fields lists filter and count facets by, each kept in an index. */
        filterable: schema.array(schema.string().min(1)),
        /** The fields lists order by, each kept in an index. */
        sortable: schema.array(schema.string().min(1)),
    }),
);
/** How callers find an object type's objects. */
export type ObjectSearch = schema.Infer<typeof ObjectSearchSchema>;

/** How callers find an object type's objects, and how a definition's roles read. */
export const ObjectSearch = Object.assign(ObjectSearchSchema, {
    /** Read a definition's roles, a role left out pointing at no field. */
    of(definition: SearchDefinition): ObjectSearch {
        return ObjectSearchSchema.parse({
            searchable: [],
            filterable: [],
            sortable: [],
            ...definition,
        });
    },
});

/** How a definition declares its query roles, a role left out pointing at no field. */
export type SearchDefinition = Partial<ObjectSearch>;

/** The field types full-text search reads. */
const SEARCHED_TYPES: ReadonlySet<string> = new Set(["string", "text", "enum"]);

/** The field types an index orders and filters by. */
const INDEXED_TYPES: ReadonlySet<string> = new Set([
    "string",
    "integer",
    "number",
    "boolean",
    "time",
    "enum",
    "reference",
    "subject",
    "fractionalIndex",
    "state",
]);

/** Objects found by query roles, each filter or sort field indexed in the objects' scope. */
export const search: Trait<ObjectSearch> = {
    key: "search",
    isDurable: true,
    options: (definition) => readSearch(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => indexes(options, table, columns),
    methods: () => ({}),
    validate: (options, object) => requireRoles(object, options),
};

/** Read a definition's query roles, absent without any. */
function readSearch(definition: Pick<ObjectDefinition, "search">): ObjectSearch | undefined {
    return definition.search === undefined ? undefined : ObjectSearch.of(definition.search);
}

/** Index each filterable or sortable field once, after the objects' scope. */
function indexes(
    options: ObjectSearch,
    table: string,
    columns: Readonly<Record<string, Column>>,
): TableConstraint[] {
    const scope = columns["scope"];
    const indexed = [...new Set([...options.filterable, ...options.sortable])];

    return indexed.map((name) => {
        const column = present(columns[name], `the column of ${name}`);

        return scope === undefined
            ? index(`${table}_${name}`).on(column)
            : index(`${table}_${name}`).on(scope, column);
    });
}

/** Refuse a role pointing at an unknown field, or a field its role cannot read. */
function requireRoles(object: ObjectType, options: ObjectSearch): void {
    const roles = [
        ["searchable", options.searchable, SEARCHED_TYPES],
        ["filterable", options.filterable, INDEXED_TYPES],
        ["sortable", options.sortable, INDEXED_TYPES],
    ] as const;
    for (const [role, names, types] of roles) {
        for (const name of names) {
            const declared = object.fields[name];
            if (declared === undefined || !types.has(declared.type)) {
                throw new TypeError(
                    `object ${object.name} declares ${name} ${role}, which is no ${role} field`,
                );
            }
        }
    }
}
