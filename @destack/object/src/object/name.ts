/** Write a camel-case or kebab-case name in kebab case. */
export function kebabCase(name: string): string {
    // split words and acronyms
    const split = name
        .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
        .replace(/([A-Z])([A-Z][a-z])/g, "$1-$2");

    return split.toLowerCase();
}

/** Write a camel-case or kebab-case name in snake case. */
export function snakeCase(name: string): string {
    return kebabCase(name).replaceAll("-", "_");
}

/** Write a kebab-case name in camel case. */
export function camelCase(name: string): string {
    return name.replace(/-([a-z0-9])/g, (_match, letter: string) => letter.toUpperCase());
}

/** Write a kebab-case name in Pascal case. */
export function pascalCase(name: string): string {
    const camel = camelCase(name);

    return camel.charAt(0).toUpperCase() + camel.slice(1);
}

/** A kebab-case name in snake case. */
export type SnakeCase<Name extends string> = Name extends `${infer Head}-${infer Tail}`
    ? `${Head}_${SnakeCase<Tail>}`
    : Name;
