import { type AsyncBuffer, parquetReadObjects } from "hyparquet";
import { ByteWriter, parquetWrite } from "hyparquet-writer";
import type { Identifier } from "@destack/schema";
import type { Entry } from "../entry/index.ts";

/** The rows of one Parquet row group: small enough that a time-pruned read stays cheap. */
const ROW_GROUP_ROWS = 8192;

/** The contents bit a span sets. */
const SPAN_BIT = 1;

/** The contents bit a metric point sets. */
export const POINT_BIT = 1 << 25;

/** The contents bit of log records without a severity, OpenTelemetry's unspecified one. */
export const UNSPECIFIED_BIT = 1 << 26;

/** The Parquet columns of a segment, one per entry field, identifiers as fixed-width bytes. */
const SCHEMA = [
    { name: "time", type: "INT64", repetition_type: "REQUIRED" },
    { name: "kind", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "REQUIRED" },
    { name: "name", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "REQUIRED" },
    { name: "duration", type: "INT64", repetition_type: "OPTIONAL" },
    { name: "instance", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "OPTIONAL" },
    { name: "source", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "REQUIRED" },
    { name: "version", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "REQUIRED" },
    { name: "trace", type: "FIXED_LEN_BYTE_ARRAY", type_length: 16, repetition_type: "OPTIONAL" },
    { name: "span", type: "FIXED_LEN_BYTE_ARRAY", type_length: 8, repetition_type: "OPTIONAL" },
    { name: "parent", type: "FIXED_LEN_BYTE_ARRAY", type_length: 8, repetition_type: "OPTIONAL" },
    { name: "status", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "REQUIRED" },
    { name: "severity", type: "INT32", repetition_type: "OPTIONAL" },
    { name: "body", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "OPTIONAL" },
    { name: "attributes", type: "BYTE_ARRAY", converted_type: "JSON", repetition_type: "REQUIRED" },
    { name: "metric", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "OPTIONAL" },
    { name: "unit", type: "BYTE_ARRAY", converted_type: "UTF8", repetition_type: "OPTIONAL" },
    { name: "value", type: "DOUBLE", repetition_type: "OPTIONAL" },
    { name: "histogram", type: "BYTE_ARRAY", converted_type: "JSON", repetition_type: "OPTIONAL" },
] as const;

/** The entries of one installation in a time window, stored as one Parquet file. */
export class Segment {
    /** The installation whose entries the segment keeps, absent for the scope's host. */
    readonly installation: Identifier<"installation"> | undefined;
    /** The entries, oldest first. */
    readonly #entries: Entry[] = [];

    /** Start an empty segment of an installation, or of the scope's host. */
    constructor(installation: Identifier<"installation"> | undefined) {
        this.installation = installation;
    }

    /** The entries, oldest first. */
    get entries(): readonly Entry[] {
        return this.#entries;
    }

    /** The earliest entry time, in Unix microseconds. */
    get from(): number {
        return this.#entries[0]!.time;
    }

    /** The latest entry time, in Unix microseconds. */
    get to(): number {
        return this.#entries.at(-1)!.time;
    }

    /** The kinds present: bit 0 for spans, bits 1 to 24 for log severities, bit 25 for points, bit 26 for logs without a severity. */
    get contents(): number {
        let bits = 0;
        for (const entry of this.#entries) {
            if (entry.kind === "span") {
                bits |= SPAN_BIT;
            } else if (entry.kind === "point") {
                bits |= POINT_BIT;
            } else {
                bits |= entry.severity === undefined ? UNSPECIFIED_BIT : 1 << entry.severity;
            }
        }

        return bits;
    }

    /** Append an entry in time order. */
    append(entry: Entry): void {
        // insert behind the last entry not later than it, usually at the end
        let position = this.#entries.length;
        while (position > 0 && this.#entries[position - 1]!.time > entry.time) {
            position--;
        }
        this.#entries.splice(position, 0, entry);
    }

    /** Encode the entries as a Parquet file, one column per entry field. */
    async encode(): Promise<Uint8Array<ArrayBuffer>> {
        // lay out one column per field, absent values as null
        const entries = this.#entries;
        const columns: Record<(typeof SCHEMA)[number]["name"], unknown[]> = {
            time: entries.map((entry) => BigInt(entry.time)),
            kind: entries.map((entry) => entry.kind),
            name: entries.map((entry) => entry.name),
            duration: entries.map((entry) =>
                entry.duration === undefined ? null : BigInt(entry.duration),
            ),
            instance: entries.map((entry) => entry.instance ?? null),
            source: entries.map((entry) => entry.source.name),
            version: entries.map((entry) => entry.source.version),
            trace: entries.map((entry) => bytes(entry.trace)),
            span: entries.map((entry) => bytes(entry.span)),
            parent: entries.map((entry) => bytes(entry.parent)),
            status: entries.map((entry) => entry.status),
            severity: entries.map((entry) => entry.severity ?? null),
            body: entries.map((entry) => entry.body ?? null),
            attributes: entries.map((entry) => JSON.stringify(entry.attributes)),
            metric: entries.map((entry) => entry.metric ?? null),
            unit: entries.map((entry) => entry.unit ?? null),
            value: entries.map((entry) => entry.value ?? null),
            histogram: entries.map((entry) =>
                entry.histogram === undefined ? null : JSON.stringify(entry.histogram),
            ),
        };

        // write them in row groups under the segment schema
        const writer = new ByteWriter();
        await parquetWrite({
            writer,
            columnData: SCHEMA.map((column) => ({
                name: column.name,
                data: columns[column.name],
                bloomFilter: column.name === "trace",
            })),
            schema: [{ name: "root", num_children: SCHEMA.length }, ...SCHEMA],
            rowGroupSize: ROW_GROUP_ROWS,
        });

        return new Uint8Array(writer.getBuffer());
    }

    /** Decode the entries of a segment file within a time window, and of one trace when given, oldest first. */
    static async decode(
        file: AsyncBuffer,
        installation: Identifier<"installation"> | undefined,
        window: { readonly from: number; readonly to: number },
        trace?: string,
    ): Promise<Entry[]> {
        // read the rows in the window, skipping row groups outside it or without the trace
        const time = { time: { $gte: BigInt(window.from), $lte: BigInt(window.to) } };
        const rows = await parquetReadObjects({
            file,
            filter:
                trace === undefined
                    ? time
                    : { $and: [time, { trace: { $eq: Uint8Array.fromHex(trace) } }] },
            useBloomFilters: trace !== undefined,
            utf8: true,
        });

        // rebuild each entry without absent fields
        return rows.map((row): Entry => {
            // read the identifiers back as hexadecimal digits
            const trace = hex(row.trace);
            const span = hex(row.span);
            const parent = hex(row.parent);

            return {
                kind: row.kind,
                name: row.name,
                time: Number(row.time),
                ...(row.duration === null ? {} : { duration: Number(row.duration) }),
                ...(installation === undefined ? {} : { installation }),
                ...(row.instance === null ? {} : { instance: row.instance }),
                source: { name: row.source, version: row.version },
                ...(trace === undefined ? {} : { trace }),
                ...(span === undefined ? {} : { span }),
                ...(parent === undefined ? {} : { parent }),
                status: row.status,
                ...(row.severity === null ? {} : { severity: row.severity }),
                ...(row.body === null ? {} : { body: row.body }),
                attributes: JSON.parse(row.attributes),
                ...(row.metric === null ? {} : { metric: row.metric }),
                ...(row.unit === null ? {} : { unit: row.unit }),
                ...(row.value === null ? {} : { value: row.value }),
                ...(row.histogram === null ? {} : { histogram: JSON.parse(row.histogram) }),
            };
        });
    }
}

/** Convert hexadecimal digits to bytes, absent as null. */
function bytes(digits: string | undefined): Uint8Array | null {
    return digits === undefined ? null : Uint8Array.fromHex(digits);
}

/** Convert bytes to hexadecimal digits, null as absent. */
function hex(value: Uint8Array | null): string | undefined {
    return value === null ? undefined : value.toHex();
}
