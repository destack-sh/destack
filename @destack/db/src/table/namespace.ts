import type { Package } from "@destack/package";

/** The longest identifier PostgreSQL stores without truncation, NAMEDATALEN minus one. */
const MAX_IDENTIFIER_LENGTH = 63;

/** The hexadecimal digits of the hash suffix: 32 bits, so two shortened names collide once in four billion. */
const HASH_LENGTH = 8;

/** The 32-bit FNV-1a offset basis. */
const FNV_OFFSET = 0x81_1c_9d_c5;

/** The 32-bit FNV prime. */
const FNV_PRIME = 0x01_00_01_93;

/** The encoder of names' UTF-8 bytes. */
const UTF8 = new TextEncoder();

/** Derive a package's SQL namespace. */
export function namespaceOf(owner: Package): string {
    const [account, name] = owner.name.slice(1).split("/");
    if (account === undefined || name === undefined) {
        throw new TypeError(`package name is not scoped: ${owner.name}`);
    }

    return `${account}__${name}`.replace(/[^a-z0-9_]/gu, "_");
}

/** Qualify a SQL identifier with its package namespace. */
export function qualify(owner: Package, name: string): string {
    // reject identifiers PostgreSQL would truncate
    const qualified = `${namespaceOf(owner)}__${name}`;
    if (qualified.length > MAX_IDENTIFIER_LENGTH) {
        throw new TypeError(
            `SQL identifier exceeds ${MAX_IDENTIFIER_LENGTH} characters: ${qualified}`,
        );
    }

    return qualified;
}

/** Name a relation within its database's namespace in a SQLite store several databases share, the name itself outside one. */
export function relation(name: string, namespace: string | undefined): string {
    return namespace === undefined ? name : `${namespace}.${name}`;
}

/** Fit a derived SQL name and a reserved suffix within the identifier limit, ending in a hash. */
export function boundedName(name: string, reserved = 0): string {
    // keep names that fit
    const limit = MAX_IDENTIFIER_LENGTH - reserved;
    if (name.length <= limit) {
        return name;
    }

    return `${name.slice(0, limit - HASH_LENGTH - 1)}_${hashName(name)}`;
}

/** Hash text into a name suffix with 32-bit FNV-1a. */
export function hashName(text: string): string {
    let hash = FNV_OFFSET;
    for (const byte of UTF8.encode(text)) {
        hash = Math.imul(hash ^ byte, FNV_PRIME) >>> 0;
    }

    return hash.toString(16).padStart(HASH_LENGTH, "0");
}
