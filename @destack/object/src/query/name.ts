import type { Query } from "@destack/sync";

/** The separator between a reference's name and one of its types, as in `subject:task`. */
const SEPARATOR = ":";

/** The names of relations, a reference to several types relating each type under its own. */
export const RelationName = {
    /** Name a reference's relation to one of its types by the type's table. */
    of(reference: string, table: string): string {
        return `${reference}${SEPARATOR}${table}`;
    },

    /** List the names a compiled query nests an include under, one per type of a reference. */
    within(query: Pick<Query, "with">, name: string): readonly string[] {
        const prefix = `${name}${SEPARATOR}`;
        const names = Object.keys(query.with ?? {}).filter((each) => each.startsWith(prefix));

        return names.length === 0 ? [name] : names;
    },
};
