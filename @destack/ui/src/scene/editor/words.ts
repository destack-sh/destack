/** Write a camelCase name as words a person reads, its first one capitalised when it leads. */
export function wordsOf(name: string, isLeading: boolean): string {
    // break before each capital and digit run, and lower the words
    const words = name
        .replaceAll(/([a-z0-9])([A-Z])/gu, "$1 $2")
        .replaceAll(/([a-zA-Z])([0-9])/gu, "$1 $2")
        .toLowerCase();

    return isLeading ? `${words.charAt(0).toUpperCase()}${words.slice(1)}` : words;
}
