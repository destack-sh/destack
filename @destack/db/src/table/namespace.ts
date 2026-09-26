import type { Package } from "@destack/package";

/** The longest identifier PostgreSQL stores without truncation, NAMEDATALEN minus one. */
const MAX_IDENTIFIER_LENGTH = 63;

/** The hexadecimal digits of a 32-bit hash that keeps shortened names distinct. */
const HASH_LENGTH = 8;

/** Derive a package's SQL namespace from its account and package name. */
export function namespaceOf(owner: Package): string {
    const [account, name] = owner.name.slice(1).split("/");

    return `${account}__${name}`.replace(/[^a-z0-9_]/g, "_");
}

/** Qualify a SQL identifier with its package namespace. */
export function qualify(owner: Package, name: string): string {
    // reject identifiers PostgreSQL would silently truncate
    const qualified = `${namespaceOf(owner)}__${name}`;
    if (qualified.length > MAX_IDENTIFIER_LENGTH) {
        throw new TypeError(
            `SQL identifier exceeds ${MAX_IDENTIFIER_LENGTH} characters: ${qualified}`,
        );
    }

    return qualified;
}

/** Qualify an optional index or key name, which must be unique across the database. */
export function constraintName<Name extends string | undefined>(owner: Package, name: Name): Name {
    return (name === undefined ? name : qualify(owner, name)) as Name;
}

/** Fit a derived SQL name within the identifier limit, replacing its tail with a hash of the whole name. */
export function boundedName(name: string): string {
    // keep names that fit
    if (name.length <= MAX_IDENTIFIER_LENGTH) {
        return name;
    }

    return `${name.slice(0, MAX_IDENTIFIER_LENGTH - HASH_LENGTH - 1)}_${hashName(name)}`;
}

/** Hash text into a short name suffix with 32-bit FNV-1a, the same on every runtime. */
export function hashName(text: string): string {
    let hash = 0x811c9dc5;
    for (const byte of new TextEncoder().encode(text)) {
        hash = Math.imul(hash ^ byte, 0x01000193) >>> 0;
    }

    return hash.toString(16).padStart(HASH_LENGTH, "0");
}
