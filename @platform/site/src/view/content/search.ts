import { schema } from "@destack/schema";

/** One searchable page or section. */
export const SearchEntry = schema.object({
    /** The collection or page containing the result. */
    context: schema.string(),
    /** The result class used for search ranking. */
    kind: schema.enum(["page", "section"]),
    /** The destination selected from search. */
    route: schema.string(),
    /** The complete searchable text. */
    text: schema.string(),
    /** The primary result title. */
    title: schema.string(),
});
/** One searchable page or section. */
export type SearchEntry = schema.Infer<typeof SearchEntry>;

/** One searchable content class. */
export type SearchEntryKind = SearchEntry["kind"];

/** Load and validate the generated full-text search index. */
export async function loadSearchEntries(): Promise<readonly SearchEntry[]> {
    // fetch the index and reject a failed response
    const response = await fetch("/search.json");
    if (!response.ok) {
        throw new Error(`cannot load search index (${response.status})`);
    }

    // validate every entry
    return schema.array(SearchEntry).parse(await response.json());
}
