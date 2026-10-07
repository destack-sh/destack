import { type AsyncBuffer, parquetReadObjects } from "hyparquet";
import { ByteWriter, parquetWrite } from "hyparquet-writer";
import { schema } from "@destack/schema";
import { KEY_INDEX_LIMIT, type KeyIndex } from "./catalog.ts";
import type { Event, EventKeyType, EventKind } from "../kind/kind.ts";
import { EventFilter } from "../query/query.ts";

/** The rows of one Parquet row group: small enough that a time-pruned read stays cheap. */
const ROW_GROUP_ROWS = 8192;

/** The Parquet column type of each query key type. */
const KEY_COLUMNS = {
    text: "STRING",
    integer: "INT64",
    real: "DOUBLE",
} as const satisfies Readonly<Record<EventKeyType, string>>;

/** The columns every segment row has, as the reader decodes them. */
const SegmentRow = schema.looseObject({
    id: schema.string(),
    source: schema.string(),
    time: schema.union([schema.bigint(), schema.number()]),
    data: schema.string(),
});

/** The events of one kind in one scope as a Parquet file in time order. */
export const SegmentFile = {
    /** Encode events as a Parquet file: one column per query key beside their identity, source, time and data. */
    async encode(kind: EventKind, events: readonly Event[]): Promise<Uint8Array<ArrayBuffer>> {
        // lay out the columns every event has, then one per query key
        const types = kind.keyTypes;
        const writer = new ByteWriter();
        await parquetWrite({
            writer,
            columnData: [
                { name: "id", data: events.map((event) => event.id), type: "STRING" },
                { name: "source", data: events.map((event) => event.source), type: "STRING" },
                { name: "time", data: events.map((event) => BigInt(event.time)), type: "INT64" },
                {
                    name: "data",
                    data: events.map((event) => JSON.stringify(event.data)),
                    type: "STRING",
                },
                ...kind.keys.map((name) => {
                    const type = types[name] ?? "text";

                    return {
                        name,
                        data: events.map((event) => {
                            const value = event.keys[name];

                            return typeof value === "number" && type === "integer"
                                ? BigInt(value)
                                : typeof value === "string" || typeof value === "number"
                                  ? value
                                  : null;
                        }),
                        type: KEY_COLUMNS[type],
                        nullable: true,
                        bloomFilter: type === "text",
                    };
                }),
            ],
            rowGroupSize: ROW_GROUP_ROWS,
        });

        return new Uint8Array(writer.getBuffer());
    },

    /** Decode the events of a segment file a filter's time range and key values select, in time order. */
    async decode(kind: EventKind, file: AsyncBuffer, filter: EventFilter): Promise<Event[]> {
        // read the rows in the range and with the values asked for, skipping row groups outside them
        const types = kind.keyTypes;
        const time = {
            ...(filter.from === undefined ? {} : { $gte: BigInt(filter.from) }),
            ...(filter.before === undefined ? {} : { $lt: BigInt(filter.before) }),
        };
        const wanted = Object.entries(EventFilter.equalities(kind, filter)).map(
            ([name, value]) => ({
                [name]: {
                    $eq:
                        types[name] === "integer" && typeof value === "number"
                            ? BigInt(value)
                            : value,
                },
            }),
        );
        const rows = await parquetReadObjects({
            file,
            filter: { $and: [{ time }, ...wanted] },
            useBloomFilters: wanted.length > 0,
            utf8: true,
        });

        // rebuild each stored event, its keys as numbers and text
        return rows.map((row) => {
            const read = SegmentRow.parse(row);
            const keys = Object.fromEntries(
                kind.keys.map((name) => {
                    const value: unknown = row[name];

                    return [
                        name,
                        typeof value === "bigint"
                            ? Number(value)
                            : typeof value === "string" || typeof value === "number"
                              ? value
                              : null,
                    ];
                }),
            );

            return {
                scope: filter.scope,
                id: read.id,
                source: read.source,
                time: Number(read.time),
                keys,
                data: schema.json().parse(JSON.parse(read.data)),
            };
        });
    },

    /** Collect the distinct values of each query key, none for a key holding more than the catalog lists. */
    keyIndex(kind: EventKind, events: readonly Event[]): KeyIndex {
        return Object.fromEntries(
            kind.keys.map((name) => {
                const values = new Set<string | number>();
                for (const event of events) {
                    const value = event.keys[name];
                    if (typeof value === "string" || typeof value === "number") {
                        values.add(value);
                    }
                }

                return [
                    name,
                    values.size > KEY_INDEX_LIMIT ? null : [...values].toSorted(compareValues),
                ];
            }),
        );
    },

    /** Report whether a segment may hold events with the key values a filter requires. */
    mayHold(kind: EventKind, sets: KeyIndex, filter: EventFilter): boolean {
        return Object.entries(EventFilter.equalities(kind, filter)).every(([name, value]) => {
            const held = sets[name];

            return held === null || held === undefined || held.includes(value);
        });
    },
};

/** Order key values: numbers before text, each in its natural order. */
function compareValues(left: string | number, right: string | number): number {
    if (typeof left === "number" && typeof right === "number") {
        return left - right;
    }

    return String(left) < String(right) ? -1 : String(left) > String(right) ? 1 : 0;
}
