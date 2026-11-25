import { ObjectError } from "../error/error.ts";
import { type Column, index, type OpenCondition, Search, type TableConstraint } from "@destack/db";
import { CHUNKS } from "../text/table.ts";
import { present, schema } from "@destack/schema";

import type { ObjectDefinition, ObjectType } from "../object/object.ts";
import type { TraitBehavior } from "./trait.ts";
import { callableTrait } from "./trait.ts";

import type {} from "@destack/package/import-meta";

/** How callers find a type's objects. */
const SearchSchema = schema.object({
    /** The fields the space's full-text index covers, the most significant first. */
    text: schema.array(schema.string().min(1)).readonly(),
    /** The fields lists filter and count facets by, each indexed. */
    filter: schema.array(schema.string().min(1)).readonly(),
    /** The fields lists order by, each indexed. */
    sort: schema.array(schema.string().min(1)).readonly(),
});
/** The fields search ranks and lists filter and sort a type's objects by. */
export type SearchFields = schema.Infer<typeof SearchSchema>;

/** The search of objects: resolving a definition's roles, expanding searches, indexing fields. */
export const Searchable = {
    /** Resolve a definition's search roles, absent without any. */
    of(definition: Pick<ObjectDefinition, "searchable">): SearchFields | undefined {
        return definition.searchable === undefined
            ? undefined
            : SearchSchema.parse({ text: [], filter: [], sort: [], ...definition.searchable });
    },

    /** Expand a condition's searches: each word in a searched column or a searched text's chunk. */
    expand(object: ObjectType, condition: OpenCondition): OpenCondition {
        // leave a type searching no text to its table's index
        const texts = (object.search?.text ?? []).filter((name) => object.text.includes(name));
        if (texts.length === 0) {
            return condition;
        }
        const columns = (object.search?.text ?? []).filter((name) => !texts.includes(name));

        return Searchable.expandSearches(condition, texts, columns.length > 0);
    },

    /** Index each filterable or sortable field once, after the objects' scope. */
    indexes(
        options: SearchFields,
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
    requireRoles(object: ObjectType, options: SearchFields): void {
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
            Searchable.expandSearches(nested, texts, isColumnSearched);
        const combined = [
            ...Searchable.readConditions(all).map(expand),
            ...Searchable.expandWords(searched, texts, isColumnSearched),
        ];

        return {
            ...rest,
            ...(combined.length === 0 ? {} : { AND: combined }),
            ...(any === undefined ? {} : { OR: Searchable.readConditions(any).map(expand) }),
            ...(negated === undefined ? {} : { NOT: expand(Searchable.readCondition(negated)) }),
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

        return Search.parse(searched).words.map((word) => ({
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

        return entry.map((nested: unknown) => Searchable.readCondition(nested));
    },

    /** Read a condition, refusing an entry that is not a condition. */
    readCondition(entry: unknown): OpenCondition {
        if (!Searchable.isCondition(entry)) {
            throw ObjectError.of("COMBINATION_REQUIRED", "combination condition is required");
        }

        return entry;
    },

    /** Report whether an entry is a condition: an object of entries. */
    isCondition(entry: unknown): entry is OpenCondition {
        return typeof entry === "object" && entry !== null && !Array.isArray(entry);
    },
};

/** How a definition declares its query roles, a role left out pointing at no field. */
export type SearchableDefinition = Partial<SearchFields>;

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
const searchableTrait: TraitBehavior<SearchFields> = {
    key: "searchable",
    storages: ["durable", "events"],
    options: (definition) => Searchable.of(definition),
    columns: () => ({}),
    constraints: (options, table, columns) => Searchable.indexes(options, table, columns),
    // index the searchable fields' text, which a list's `SEARCH` condition matches
    table: (options) => ({ fullText: options.text }),
    methods: () => ({}),
    validate: (options, object) => Searchable.requireRoles(object, options),
};

/** Find objects by text, and index the fields callers filter and sort by. */
export const searchable = callableTrait("searchable", searchableTrait);
