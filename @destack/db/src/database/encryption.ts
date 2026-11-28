import type { Encryption } from "../table/column.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The dialect of an encrypted value's text form, as encrypted columns store text. */
export const ENCRYPTED_DIALECT: Dialect = "sqlite";

/** Encrypts and decrypts a database's sensitive columns at rest. */
export interface ColumnCipher {
    /** Encrypt a column's driver value as text bound to the column. */
    readonly encrypt: (value: unknown, column: string, encryption: Encryption) => string;
    /** Decrypt an encrypted column value back into its driver value. */
    readonly decrypt: (encrypted: unknown, column: string) => unknown;
}

/** The ciphers of sensitive columns, as queries encrypt and decrypt them. */
export const ColumnCipher = {
    /** Require the cipher of a sensitive column, which a database declaring one opens with. */
    require(
        cipher: ColumnCipher | undefined,
        column: { readonly definition: { readonly name: string } },
    ): ColumnCipher {
        if (cipher === undefined) {
            throw new TypeError(
                `cannot encrypt sensitive column '${column.definition.name}': the database has no key`,
            );
        }

        return cipher;
    },
};
