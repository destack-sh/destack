import { expect, test } from "@destack/test";
import { identifier } from "@destack/schema";
import type { Entry } from "../src/entry/index.ts";
import { Segment } from "../src/segment/index.ts";

/** The installation emitting the entries. */
const notes = identifier("installation").parse("installation-01996ab0-0000-7000-8000-000000000005");

/** A span and a log record inside it, and a later warning without a trace. */
const entries: Entry[] = [
    {
        kind: "span",
        name: "note.render",
        time: 1_000,
        duration: 250,
        installation: notes,
        instance: "instance-1",
        source: { name: "@example/notes", version: "2026.9.0" },
        trace: "0192f3a4b5c600000000000000000001",
        span: "00000000000000a1",
        parent: "00000000000000a0",
        status: "error",
        attributes: { blocks: 12, tags: ["draft", "shared"] },
    },
    {
        kind: "log",
        name: "note.saved",
        time: 1_100,
        installation: notes,
        source: { name: "@example/notes", version: "2026.9.0" },
        trace: "0192f3a4b5c600000000000000000001",
        span: "00000000000000a1",
        status: "unset",
        severity: 9,
        attributes: { length: 5, isConflict: false },
    },
    {
        kind: "log",
        name: "note.conflict",
        time: 2_000,
        installation: notes,
        source: { name: "@example/notes", version: "2026.9.0" },
        status: "unset",
        severity: 13,
        attributes: {},
    },
];

test("roundtrip a segment's entries through Parquet and read a time window and a trace of them", async () => {
    // append out of order, then encode
    const segment = new Segment(notes);
    for (const entry of [entries[2]!, entries[0]!, entries[1]!]) {
        segment.append(entry);
    }
    const file = (await segment.encode()).buffer;

    // read every entry, then only the window around the span
    const every = await Segment.decode(file, notes, { from: 0, to: 10_000 });
    const early = await Segment.decode(file, notes, { from: 1_000, to: 1_500 });
    const traced = await Segment.decode(
        file,
        notes,
        { from: 0, to: 10_000 },
        "0192f3a4b5c600000000000000000001",
    );

    // keep time order, every field, the severities present and the bounds
    expect({
        every,
        early,
        traced,
        bounds: [segment.from, segment.to],
        contents: segment.contents,
    }).toEqual({
        every: entries,
        early: entries.slice(0, 2),
        traced: entries.slice(0, 2),
        bounds: [1_000, 2_000],
        contents: 1 | (1 << 9) | (1 << 13),
    });
});
