import { binary, defineTable, index, integer, json, text, type Select } from "@destack/db";
import { schema } from "@destack/schema";

/** The most distinct values a segment lists per query key before the catalog drops the set. */
export const KEY_INDEX_LIMIT = 64;

/** The bits a bloom filter spends per value: about one false positive in a hundred. */
const FILTER_BITS_PER_VALUE = 10;

/** The hashes a bloom filter sets per value, ten bits times ln 2 rounded. */
const FILTER_HASHES = 7;

/** The 32-bit FNV-1a offset basis and prime. */
const FNV = { offset: 0x811c9dc5, prime: 0x01000193 } as const;

/** The encoder of hashed texts, shared by every hash. */
const ENCODER = new TextEncoder();

/** A set of texts as a bloom filter, about one false positive in a hundred. */
export const BloomFilter = {
    /** Build the filter of some texts. */
    build(values: readonly string[]): Uint8Array<ArrayBuffer> {
        // size the bits to the texts, a byte at least
        const bits = new Uint8Array(
            Math.max(1, Math.ceil((values.length * FILTER_BITS_PER_VALUE) / 8)),
        );
        for (const value of values) {
            for (const position of bitPositions(value, bits.length * 8)) {
                bits[position >>> 3] = (bits[position >>> 3] ?? 0) | (1 << (position & 7));
            }
        }

        return bits;
    },

    /** Report whether a filter may contain a text. */
    mayHold(bits: Uint8Array, value: string): boolean {
        return bitPositions(value, bits.length * 8).every(
            (position) => ((bits[position >>> 3] ?? 0) & (1 << (position & 7))) !== 0,
        );
    },
};

/** The distinct values of each query key in a segment, null for a key with too many to list. */
export const KeyIndex = schema.record(
    schema.string(),
    schema.array(schema.union([schema.string(), schema.number()])).nullable(),
);
/** The distinct values of each query key in a segment. */
export type KeyIndex = schema.Infer<typeof KeyIndex>;

/** One flushed segment of a kind's events in a scope: the catalog queries choose segments by. */
export const eventSegment = defineTable(
    "segment",
    {
        /** The segment's identity, the name of its file. */
        id: text("id").primaryKey(),
        /** The event kind, by its package and name. */
        kind: text("kind").notNull(),
        /** The scope of its events. */
        scope: text("scope").notNull(),
        /** The earliest event time, in Unix microseconds. */
        from: integer("from").notNull(),
        /** The latest event time, in Unix microseconds. */
        to: integer("to").notNull(),
        /** The event count. */
        rows: integer("rows").notNull(),
        /** The file size, in bytes. */
        bytes: integer("bytes").notNull(),
        /** The distinct values of each query key. */
        keys: json("keys", KeyIndex).notNull(),
        /** The filter of the event identifiers, read only to find one event. */
        ids: binary("ids").notNull(),
        /** The filter of the three-character runs of the searched words, read only by searches. */
        grams: binary("grams").notNull(),
        /** The bucket key of the file. */
        file: text("file").notNull(),
        /** The SHA-256 digest of the file, in hexadecimal. */
        digest: text("digest").notNull(),
        /** The chaining digest of the scope's previous segment of the kind, absent at first. */
        previous: text("previous"),
        /** When the segment was flushed, in Unix milliseconds. */
        flushedAt: integer("flushed_at").notNull(),
    },
    {
        constraints: (segment) => [
            index("segment_scope_time").on(segment.kind, segment.scope, segment.to),
            index("segment_scope_flushed").on(segment.kind, segment.scope, segment.flushedAt),
        ],
    },
);

/** List the bit positions of a text, by double hashing two FNV-1a hashes. */
function bitPositions(value: string, size: number): number[] {
    // hash the text twice, the second hash seeded by the first
    const bytes = ENCODER.encode(value);
    const first = fnv(bytes, FNV.offset);
    const second = (fnv(bytes, first) | 1) >>> 0;

    return Array.from({ length: FILTER_HASHES }, (_, round) => (first + round * second) % size);
}

/** Hash bytes with 32-bit FNV-1a from a seed. */
function fnv(bytes: Uint8Array, seed: number): number {
    let hash = seed;
    for (const byte of bytes) {
        hash = Math.imul(hash ^ byte, FNV.prime) >>> 0;
    }

    return hash;
}

/** The catalog columns reads choose segments by, without the identifier and gram filters. */
export const chosenSegment = {
    id: eventSegment.id,
    kind: eventSegment.kind,
    scope: eventSegment.scope,
    from: eventSegment.from,
    to: eventSegment.to,
    rows: eventSegment.rows,
    bytes: eventSegment.bytes,
    keys: eventSegment.keys,
    file: eventSegment.file,
    digest: eventSegment.digest,
    previous: eventSegment.previous,
    flushedAt: eventSegment.flushedAt,
};

/** A catalogued segment as reads choose it, without its identifier and gram filters. */
export type ChosenSegment = Omit<Select<typeof eventSegment>, "ids" | "grams">;
