export type SearchEntry = {
    /// The collection or page containing the result.
    context: string;

    /// The destination selected from search.
    route: string;

    /// The complete searchable text.
    text: string;

    /// The primary result title.
    title: string;
};

/// Load and validate the generated full-text search index.
export async function loadSearchEntries(): Promise<readonly SearchEntry[]> {
    const response = await fetch("/search.json");
    if (!response.ok) {
        throw new Error(`cannot load search index (${response.status})`);
    }

    const entries: unknown = await response.json();
    if (!Array.isArray(entries) || !entries.every(isSearchEntry)) {
        throw new Error("invalid search index");
    }

    return entries;
}

/// Return whether one value is a complete search entry.
function isSearchEntry(value: unknown): value is SearchEntry {
    if (typeof value !== "object" || value == null) {
        return false;
    }

    const entry = value as Record<string, unknown>;

    return (
        typeof entry.context === "string" &&
        typeof entry.route === "string" &&
        typeof entry.text === "string" &&
        typeof entry.title === "string"
    );
}
