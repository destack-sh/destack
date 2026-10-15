/** A kebab-case name in snake case. */
export type SnakeCase<Name extends string> = Name extends `${infer Head}-${infer Tail}`
    ? `${Head}_${SnakeCase<Tail>}`
    : Name;

/** The casings of declared names: kebab, snake, camel and pascal case. */
export const Casing = {
    /** Write a camel-case or kebab-case name in kebab case. */
    kebab(name: string): string {
        // split words and acronyms
        const split = name
            .replace(/([a-z0-9])([A-Z])/gu, "$1-$2")
            .replace(/([A-Z])([A-Z][a-z])/gu, "$1-$2");

        return split.toLowerCase();
    },

    /** Write a camel-case or kebab-case name in snake case. */
    snake(name: string): string {
        return Casing.kebab(name).replaceAll("-", "_");
    },

    /** Write a kebab-case name in camel case. */
    camel(name: string): string {
        return name.replace(/-([a-z0-9])/gu, (_match, letter: string) => letter.toUpperCase());
    },

    /** Write a kebab-case name in Pascal case. */
    pascal(name: string): string {
        const camel = Casing.camel(name);

        return camel.charAt(0).toUpperCase() + camel.slice(1);
    },
};
