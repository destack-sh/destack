import {
    type Column,
    index,
    type OpenCondition,
    SearchText,
    type TableConstraint,
} from "@destack/db";
import { CHUNKS } from "../text/chunk.ts";
import { defineSchema, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
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

    /** Expand a condition's searches: each word in a searched column or a searched text's chunk. */
    expand(object: ObjectType, condition: OpenCondition): OpenCondition {
        // leave a type searching no text to its table's index
        const texts = (object.search?.searchable ?? []).filter((name) =>
            object.text.includes(name),
        );
        if (texts.length === 0) {
            return condition;
        }
        const columns = (object.search?.searchable ?? []).filter((name) => !texts.includes(name));

        return ObjectSearch.expandSearches(condition, texts, columns.length > 0);
    },

    /** Read a definition's query roles, absent without any. */
    readSearch(definition: Pick<ObjectDefinition, "search">): ObjectSearch | undefined {
        return definition.search === undefined ? undefined : ObjectSearch.of(definition.search);
    },

    /** Index each filterable or sortable field once, after the objects' scope. */
    indexes(
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
    },

    /** Refuse a role pointing at an unknown field, or a field its role cannot read. */
    requireRoles(object: ObjectType, options: ObjectSearch): void {
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
                        `object ${object.name} declares ${name} ${role}, which is not a ${role} field`,
                    );
                }
            }
        }
    },

    /** Expand the searches of a condition and the conditions it combines, related types' aside. */
    expandSearches(
        condition: OpenCondition,
        texts: readonly string[],
        isColumnSearched: boolean,
    ): OpenCondition {
        // expand the conditions a condition combines and negates, keeping its other entries
        const { SEARCH: searched, AND: all, OR: any, NOT: negated, ...rest } = condition;
        const expand = (nested: OpenCondition) =>
            ObjectSearch.expandSearches(nested, texts, isColumnSearched);
        const combined = [
            ...ObjectSearch.readConditions(all).map(expand),
            ...ObjectSearch.expandWords(searched, texts, isColumnSearched),
        ];

        return {
            ...rest,
            ...(combined.length === 0 ? {} : { AND: combined }),
            ...(any === undefined ? {} : { OR: ObjectSearch.readConditions(any).map(expand) }),
            ...(negated === undefined ? {} : { NOT: expand(ObjectSearch.readCondition(negated)) }),
        };
    },

    /** Expand a search into each word in a searched column or a searched text's chunk. */
    expandWords(
        searched: OpenCondition[string] | undefined,
        texts: readonly string[],
        isColumnSearched: boolean,
    ): OpenCondition[] {
        // keep no words without a search, and refuse a search without text
        if (searched === undefined) {
            return [];
        } else if (typeof searched !== "string") {
            throw new CallError("BAD_REQUEST", { message: "a search needs text" });
        }

        return SearchText.query(searched).map((word) => ({
            OR: [
                ...(isColumnSearched ? [{ SEARCH: word }] : []),
                { [CHUNKS]: { field: { in: [...texts] }, SEARCH: word } },
            ],
        }));
    },

    /** Read the conditions a combination lists, refusing an entry that is not a condition. */
    readConditions(entry: OpenCondition[string] | undefined): OpenCondition[] {
        if (entry === undefined) {
            return [];
        } else if (!Array.isArray(entry)) {
            throw new CallError("BAD_REQUEST", { message: "a combination needs conditions" });
        }

        return entry.map((nested: unknown) => ObjectSearch.readCondition(nested));
    },

    /** Read a condition, refusing an entry that is not a condition. */
    readCondition(entry: unknown): OpenCondition {
        if (!ObjectSearch.isCondition(entry)) {
            throw new CallError("BAD_REQUEST", { message: "a combination needs conditions" });
        }

        return entry;
    },

    /** Report whether an entry is a condition: an object of entries. */
    isCondition(entry: unknown): entry is OpenCondition {
        return typeof entry === "object" && entry !== null && !Array.isArray(entry);
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
    "object",
    "fractionalIndex",
    "state",
]);

/** Objects found by query roles, each filter or sort field indexed in the objects' scope. */
export const search: Trait<ObjectSearch> = {
    key: "search",
    storages: ["durable", "events"],
    options: (definition) => ObjectSearch.readSearch(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => ObjectSearch.indexes(options, table, columns),
    // index the searchable fields' text, which a list's `SEARCH` condition matches
    table: (options) => ({ fullText: options.searchable }),
    methods: () => ({}),
    validate: (options, object) => ObjectSearch.requireRoles(object, options),
};
