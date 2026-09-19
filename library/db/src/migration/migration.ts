/** Committed SQL that advances one named database schema. */
export interface Migration {
    /** The ordered migration directory name. */
    readonly name: string;
    /** The SHA-256 checksum of the SQL file. */
    readonly checksum: string;
    /** SQL statements in execution order. */
    readonly statements: readonly string[];
}
