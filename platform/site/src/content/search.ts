/** One searchable content class. */
export type SearchEntryKind = "module" | "page" | "rule" | "section" | "symbol";

/** One searchable page, section, module, or symbol. */
export type SearchEntry = {
    /** The collection or page containing the result. */
    context: string;

    /** The result class used for search ranking. */
    kind: SearchEntryKind;

    /** The destination selected from search. */
    route: string;

    /** The complete searchable text. */
    text: string;

    /** The primary result title. */
    title: string;
};

/** Load and validate the generated full-text search index. */
export async function loadSearchEntries(): Promise<readonly SearchEntry[]> {
    // fetch the index and reject a failed response
    const response = await fetch("/search.json");
    if (!response.ok) {
        throw new Error(`cannot load search index (${response.status})`);
    }

    // validate every entry
    const entries: unknown = await response.json();
    if (!Array.isArray(entries) || !entries.every(isSearchEntry)) {
        throw new Error("invalid search index");
    }

    return entries;
}

/** Return whether one value is a complete search entry. */
function isSearchEntry(value: unknown): value is SearchEntry {
    if (typeof value !== "object" || value == null) {
        return false;
    }

    const entry = value as Record<string, unknown>;

    return (
        typeof entry.context === "string" &&
        (entry.kind === "module" ||
            entry.kind === "page" ||
            entry.kind === "rule" ||
            entry.kind === "section" ||
            entry.kind === "symbol") &&
        typeof entry.route === "string" &&
        typeof entry.text === "string" &&
        typeof entry.title === "string"
    );
}
