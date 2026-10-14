import { DatabaseError } from "../error/error.ts";

/** What splits text into words: any run of characters that are not letters or digits. */
const SEPARATOR = /[^\p{L}\p{N}]+/u;

/** The characters of a gram, three as trigram indexes take them. */
const GRAM_LENGTH = 3;

/** The segmenter splitting a word into the characters people read. */
const GRAPHEMES = new Intl.Segmenter(undefined, { granularity: "grapheme" });

/** The most words a search reads, a long sentence. */
const MAX_SEARCH_WORDS = 32;

/** Text as search reads it, alike in SQLite, PostgreSQL and memory: case-folded words. */
export const SearchText = {
    /** Split text into its lowercase words, empty for text without words. */
    words(text: string): string[] {
        return text
            .toLowerCase()
            .split(SEPARATOR)
            .filter((word) => word.length > 0);
    },

    /** Read a search's words, refusing a search of none or of too many. */
    query(text: string): string[] {
        const words = SearchText.words(text);
        if (words.length === 0 || words.length > MAX_SEARCH_WORDS) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `a search needs 1 to ${MAX_SEARCH_WORDS} words, not ${words.length}: ${text}`,
            );
        }

        return words;
    },

    /** Report whether some texts contain each query word as the prefix of a word. */
    matches(texts: readonly unknown[], query: readonly string[]): boolean {
        const words = texts.flatMap((text) =>
            typeof text === "string" ? SearchText.words(text) : [],
        );

        return query.every((prefix) => words.some((word) => word.startsWith(prefix)));
    },

    /** List a word's runs of three characters, which a segment's gram filter keeps. */
    grams(word: string): string[] {
        const characters = Array.from(GRAPHEMES.segment(word), (piece) => piece.segment);

        return characters.length < GRAM_LENGTH
            ? []
            : characters
                  .slice(0, characters.length - GRAM_LENGTH + 1)
                  .map((_, start) => characters.slice(start, start + GRAM_LENGTH).join(""));
    },

    /** Write a query's words as an FTS5 match: each a quoted prefix, all required. */
    fts5(query: readonly string[]): string {
        return query.map((word) => `"${word}"*`).join(" ");
    },

    /** Write a query's words as a PostgreSQL tsquery: each a quoted prefix, all required. */
    tsquery(query: readonly string[]): string {
        return query.map((word) => `'${word}':*`).join(" & ");
    },
};
