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
