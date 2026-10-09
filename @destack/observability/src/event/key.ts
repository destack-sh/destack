/** The value of an event key or of an entry of a map key, as groups and alerts read them. */
export const EventKey = {
    /** Read a key's value, or the entry of a map key a dotted name such as `attributes.http.route` names, null where absent. */
    read(keys: Readonly<Record<string, unknown>>, name: string): string | number | null {
        // read a key itself, else the entry after the first dot of a map key
        const dot = name.indexOf(".");
        const map = dot === -1 ? undefined : keys[name.slice(0, dot)];
        const value: unknown =
            name in keys || typeof map !== "object" || map === null
                ? keys[name]
                : Reflect.get(map, name.slice(dot + 1));

        // keep text and numbers, writing a flag as text
        return typeof value === "string" || typeof value === "number"
            ? value
            : typeof value === "boolean"
              ? String(value)
              : null;
    },
};
