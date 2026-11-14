import type { DriverStatement } from "../sql/index.ts";

/** The span path of a statement run outside any span. */
const NO_PATH = "(none)";

/** The offset basis of 32-bit FNV-1a. */
const FNV_OFFSET = 0x81_1c_9d_c5;

/** The prime of 32-bit FNV-1a. */
const FNV_PRIME = 0x01_00_01_93;

/** One statement text's totals under one span path, as `pg_stat_statements` keeps them. */
export interface StatementRecord {
    /** The span path that ran it, such as `label/create>destack.transaction.run[main:database]`. */
    readonly path: string;
    /** The statement's identity: an FNV-1a hash of its text, in hexadecimal. */
    readonly query: string;
    /** Its operation and tables, as OpenTelemetry's `db.query.summary` reports them. */
    readonly summary: string;
    /** The runs. */
    count: number;
    /** The host seconds the runs took. */
    seconds: number;
    /** The rows the runs returned or changed. */
    rows: number;
}

/** A database's statement records by span path and text, as `pg_stat_statements` keeps them. */
export class StatementRecords {
    /** The records by span path, then by statement text. */
    readonly #records = new Map<string, Map<string, StatementRecord>>();

    /** Add a statement's run to its record under the span path running it. */
    add(path: string | undefined, statement: DriverStatement, seconds: number, rows: number): void {
        // find the span path's records
        const under = path ?? NO_PATH;
        const texts = this.#records.get(under) ?? new Map<string, StatementRecord>();
        this.#records.set(under, texts);

        // add the run to the text's record, made at its first run
        const { text } = statement;
        const record = texts.get(text) ?? {
            path: under,
            query: fnv1a(text),
            summary: [statement.operation, ...statement.tables].join(" "),
            count: 0,
            seconds: 0,
            rows: 0,
        };
        texts.set(text, record);
        record.count += 1;
        record.seconds += seconds;
        record.rows += rows;
    }

    /** List the records, the costliest first. */
    list(): readonly StatementRecord[] {
        const records = [...this.#records.values()].flatMap((texts) => [...texts.values()]);

        return records.toSorted((left, right) => right.seconds - left.seconds);
    }
}

/** Hash a text with 32-bit FNV-1a, in hexadecimal. */
function fnv1a(text: string): string {
    let hash = FNV_OFFSET;
    for (let index = 0; index < text.length; index += 1) {
        hash = Math.imul(hash ^ text.charCodeAt(index), FNV_PRIME) >>> 0;
    }

    return hash.toString(16).padStart(8, "0");
}
