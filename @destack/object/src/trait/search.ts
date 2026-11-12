import { ObjectError } from "../error/error.ts";
import {
    type Column,
    index,
    type OpenCondition,
    SearchText,
    type TableConstraint,
} from "@destack/db";
import { CHUNKS } from "../text/table.ts";
import { present, schema } from "@destack/schema";

import type { ObjectDefinition, ObjectType } from "../object/object.ts";
import type { TraitBehavior } from "./trait.ts";
import { callableTrait } from "./trait.ts";

import type {} from "@destack/package/import-meta";

/** How callers find a type's objects. */
const ObjectSearchSchema = schema.object({
    /** The fields the space's full-text index covers, the most significant first. */
    text: schema.array(schema.string().min(1)).readonly(),
    /** The fields lists filter and count facets by, each indexed. */
    filter: schema.array(schema.string().min(1)).readonly(),
    /** The fields lists order by, each indexed. */
    sort: schema.array(schema.string().min(1)).readonly(),
});
/** How callers find an object type's objects. */
export type ObjectSearch = schema.Infer<typeof ObjectSearchSchema>;

/** How callers find an object type's objects, and how a definition's roles read. */
export const ObjectSearch = Object.assign(ObjectSearchSchema, {
    /** Read a definition's roles, a role left out pointing at no field. */
    of(definition: SearchDefinition): ObjectSearch {
        return ObjectSearchSchema.parse({
            text: [],
            filter: [],
            sort: [],
            ...definition,
        });
    },

    /** Expand a condition's searches: each word in a searched column or a searched text's chunk. */
    expand(object: ObjectType, condition: OpenCondition): OpenCondition {
        // leave a type searching no text to its table's index
        const texts = (object.searchable?.text ?? []).filter((name) => object.text.includes(name));
        if (texts.length === 0) {
            return condition;
        }
        const columns = (object.searchable?.text ?? []).filter((name) => !texts.includes(name));

        return ObjectSearch.expandSearches(condition, texts, columns.length > 0);
    },

    /** Read a definition's query roles, absent without any. */
    readSearch(definition: Pick<ObjectDefinition, "searchable">): ObjectSearch | undefined {
        return definition.searchable === undefined
            ? undefined
            : ObjectSearch.of(definition.searchable);
    },

    /** Index each filterable or sortable field once, after the objects' scope. */
    indexes(
        options: ObjectSearch,
        table: string,
        columns: Readonly<Record<string, Column>>,
    ): TableConstraint[] {
        const scope = columns["scope"];
        const indexed = [...new Set([...options.filter, ...options.sort])];

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
            ["text", options.text, SEARCHED_TYPES],
            ["filter", options.filter, INDEXED_TYPES],
            ["sort", options.sort, INDEXED_TYPES],
        ] as const;
        for (const [role, names, types] of roles) {
            for (const name of names) {
                const declared = object.fields[name];
                if (declared === undefined || !types.has(declared.type)) {
                    throw new TypeError(
                        `object ${object.name} declares ${name} '${role}', which is not a '${role}' field`,
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
        // expand the conditions a condition combines and negates, with its other entries
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
        // match no words without a search, and refuse a search without text
        if (searched === undefined) {
            return [];
        } else if (typeof searched !== "string") {
            throw ObjectError.of("TEXT_REQUIRED", "search text is required");
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
            throw ObjectError.of("COMBINATION_REQUIRED", "combination condition is required");
        }

        return entry.map((nested: unknown) => ObjectSearch.readCondition(nested));
    },

    /** Read a condition, refusing an entry that is not a condition. */
    readCondition(entry: unknown): OpenCondition {
        if (!ObjectSearch.isCondition(entry)) {
            throw ObjectError.of("COMBINATION_REQUIRED", "combination condition is required");
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
    "blob",
    "state",
]);

/** Objects found by query roles, each filter or sort field indexed in the objects' scope. */
const searchTrait: TraitBehavior<ObjectSearch> = {
    key: "searchable",
    storages: ["durable", "events"],
    options: (definition) => ObjectSearch.readSearch(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => ObjectSearch.indexes(options, table, columns),
    // index the searchable fields' text, which a list's `SEARCH` condition matches
    table: (options) => ({ fullText: options.text }),
    methods: () => ({}),
    validate: (options, object) => ObjectSearch.requireRoles(object, options),
};

/** Find objects by text, and index the fields callers filter and sort by. */
export const searchable = callableTrait("searchable", searchTrait);
