import { SQL, sql, type SQLChunk, type SQLWrapper } from "drizzle-orm";
import type { Dialect } from "./dialect.ts";

/** SQL expressions implementing the same operation in each dialect. */
class DialectExpression implements SQLWrapper {
    /** The SQL expression for each supported dialect. */
    readonly expressions: Readonly<Record<Dialect, SQL>>;

    /** Retain both dialect implementations. */
    constructor(expressions: Readonly<Record<Dialect, SQL>>) {
        this.expressions = expressions;
    }

    /** Reject compilation without a selected database dialect. */
    getSQL(): SQL {
        throw new TypeError("select a database dialect before compiling this expression");
    }
}

/** Declare an expression with explicit SQL implementations for both dialects. */
export function dialectSQL<Value = unknown>(
    expressions: Readonly<Record<Dialect, SQL>>,
): SQL<Value> {
    return sql<Value>`${new DialectExpression(expressions)}`;
}

/** Select dialect expressions while retaining SQL decoders and aliases. */
export function compileExpression<Value>(
    expression: SQL<Value>,
    dialect: Dialect,
    transform?: (chunk: SQLChunk) => SQLChunk,
): SQL<Value> {
    const chunks = expression.queryChunks.map((chunk) => compileChunk(chunk, dialect, transform));
    const compiled = Object.assign(new SQL<Value>(chunks), expression, { queryChunks: chunks });

    return transform ? (transform(compiled) as SQL<Value>) : compiled;
}

/** Select dialect expressions in one SQL fragment. */
function compileChunk(
    chunk: SQLChunk,
    dialect: Dialect,
    transform?: (chunk: SQLChunk) => SQLChunk,
): SQLChunk {
    // select the dialect's expression
    if (chunk instanceof DialectExpression) {
        return compileExpression(chunk.expressions[dialect], dialect, transform);
    }
    // compile nested fragments
    else if (chunk instanceof SQL) {
        return compileExpression(chunk, dialect, transform);
    }
    // compile each listed chunk
    else if (Array.isArray(chunk)) {
        return chunk.map((value) => compileChunk(value, dialect, transform));
    }
    // transform plain chunks
    else {
        return transform ? transform(chunk) : chunk;
    }
}
