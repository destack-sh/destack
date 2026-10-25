import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** The refusal of a logged write outside a transaction, as the change triggers word it. */
const OUTSIDE_TRANSACTION =
    /a logged write of .+? runs outside a transaction: run it in a transaction/u;

/** A unique key a write violated: its table and columns as the table declares them. */
export const ViolatedKey = schema.object({
    /** The table's declared name. */
    table: schema.string(),
    /** The key's columns, by their property names. */
    columns: schema.array(schema.string()),
});
/** A unique key a write violated. */
export type ViolatedKey = schema.Infer<typeof ViolatedKey>;

/** The declared names of a database's tables by SQL name: each table's name and its columns'. */
export type DeclaredNames = ReadonlyMap<
    string,
    { readonly name: string; readonly columns: ReadonlyMap<string, string> }
>;

/** The database package's failures, each reason with the call error code it answers. */
export const DatabaseError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    INVALID_QUERY: { code: "BAD_REQUEST" },
    INVALID_RECORD: { code: "BAD_REQUEST" },

    // conflict
    BROKEN_REFERENCE: { code: "CONFLICT" },
    DUPLICATE: { code: "CONFLICT", data: ViolatedKey.nullable() },

    // gone
    CHANGES_COMPACTED: { code: "GONE" },
    STALE_EPOCH: { code: "GONE" },

    // internal server error
    CONTROLLER_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    FOLLOW_ENDED: { code: "INTERNAL_SERVER_ERROR" },
    INVALID_MIGRATION: { code: "INTERNAL_SERVER_ERROR" },
    MIGRATION_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    NOT_APPLIED: { code: "INTERNAL_SERVER_ERROR" },
    NO_CHANNEL: { code: "INTERNAL_SERVER_ERROR" },
    PLAN_CHANGED: { code: "INTERNAL_SERVER_ERROR" },
    QUERY_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    TRANSACTION_CLOSED: { code: "INTERNAL_SERVER_ERROR" },
    TRANSACTION_REQUIRED: { code: "INTERNAL_SERVER_ERROR" },
    TREE_NOT_FOUND: { code: "INTERNAL_SERVER_ERROR" },

    // service unavailable
    CONCURRENT_UPDATE: { code: "SERVICE_UNAVAILABLE" },
    CONNECTION_CLOSED: { code: "SERVICE_UNAVAILABLE" },
    CONNECTION_TIMEOUT: { code: "SERVICE_UNAVAILABLE" },
    CONTENT_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },
    OWNER_CHANGED: { code: "SERVICE_UNAVAILABLE" },
    SERIALIZATION_FAILURE: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of database failures. */
export type DatabaseReason = ReasonOf<typeof DatabaseError>;

/** Read a driver's violated unique key by its declared names, null for an expression key. */
export function violatedKeyOf(error: Error, declared: DeclaredNames): ViolatedKey | null {
    // read SQLite's "UNIQUE constraint failed" message, with namespaced tables
    const failed = /^UNIQUE constraint failed: (.+)$/u.exec(error.message)?.[1];
    if (failed !== undefined) {
        const named = failed.split(", ").map((entry) => entry.split("."));
        const tables = new Set(named.map((path) => path.at(-2)));
        const [table] = tables;

        return tables.size === 1 && table !== undefined
            ? declaredKey(
                  declared,
                  table,
                  named.map((path) => path.at(-1) ?? ""),
              )
            : null;
    }

    // read PostgreSQL's table and key detail, none for an expression key
    const table = "table_name" in error ? error.table_name : undefined;
    const detail = "detail" in error ? error.detail : undefined;
    const keys = typeof detail === "string" ? /^Key \((.*?)\)=\(/u.exec(detail)?.[1] : undefined;

    return typeof table === "string" && keys !== undefined && !keys.includes("(")
        ? declaredKey(declared, table, keys.split(", "))
        : null;
}

/** Reject an unhandled variant. */
export function assertNever(value: never): never {
    throw new TypeError(`unhandled database variant: ${String(value)}`);
}

/** Classify a failed statement as a conflict, duplicate, broken reference or failed check. */
export function classifyError(error: unknown, declared: DeclaredNames): unknown {
    // return classified and non-driver failures as they are
    if (DatabaseError.is(error) || !(error instanceof Error)) {
        return error;
    }

    // classify by PostgreSQL state or SQLite message
    const code = errorCode(error);
    if (
        code === "40001" ||
        code === "40P01" ||
        error.message.includes("database is locked") ||
        error.message.includes("database table is locked")
    ) {
        return DatabaseError.of(
            "SERIALIZATION_FAILURE",
            "a concurrent transaction conflicted with this one",
            {
                cause: error,
            },
        );
    } else if (code === "23505" || error.message.includes("UNIQUE constraint failed")) {
        return DatabaseError.of("DUPLICATE", "a record with the same unique key already exists", {
            cause: error,
            data: violatedKeyOf(error, declared),
        });
    } else if (
        code === "23503" ||
        code === "23001" ||
        error.message.includes("FOREIGN KEY constraint failed")
    ) {
        return DatabaseError.of(
            "BROKEN_REFERENCE",
            "the change would leave a reference to a missing record",
            {
                cause: error,
            },
        );
    } else if (code === "25P01" || OUTSIDE_TRANSACTION.test(error.message)) {
        return DatabaseError.of(
            "TRANSACTION_REQUIRED",
            OUTSIDE_TRANSACTION.exec(error.message)?.[0] ?? error.message,
            { cause: error },
        );
    } else if (code === "23514" || error.message.includes("CHECK constraint failed")) {
        return DatabaseError.of("INVALID_RECORD", "a record fails a declared check", {
            cause: error,
        });
    }

    return error;
}

/** Read a driver failure's code, absent for a failure without one. */
export function errorCode(error: unknown): string | undefined {
    return error instanceof Error && "code" in error && typeof error.code === "string"
        ? error.code
        : undefined;
}

/** Report whether a failure only reports a transaction an earlier statement's failure aborted. */
export function isAborted(failure: unknown): boolean {
    return errorCode(failure) === "25P02";
}

/** Report whether an error is or wraps a failure. */
export function wraps(error: unknown, failure: unknown): boolean {
    for (let current = error; current !== undefined;) {
        if (current === failure) {
            return true;
        }
        current = current instanceof Error ? current.cause : undefined;
    }

    return false;
}

/** Name a key of SQL columns as its table declares them, null for an undeclared table. */
function declaredKey(
    declared: DeclaredNames,
    table: string,
    columns: readonly string[],
): ViolatedKey | null {
    // name the table and each column as the table declares them
    const names = declared.get(table);
    const properties = columns.map((column) => names?.columns.get(column));
    if (names === undefined || properties.some((property) => property === undefined)) {
        return null;
    }

    return { table: names.name, columns: properties.flatMap((property) => property ?? []) };
}
