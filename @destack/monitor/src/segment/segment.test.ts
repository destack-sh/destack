import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import type { Entry } from "../entry/index.ts";
import { Segment } from "./index.ts";

/** The installation emitting the entries. */
const notes = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-000000000005");

/** The span. */
const span: Entry = {
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
};

/** The log record inside the span. */
const saved: Entry = {
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
};

/** The later warning without a trace. */
const conflict: Entry = {
    kind: "log",
    name: "note.conflict",
    time: 2_000,
    installation: notes,
    source: { name: "@example/notes", version: "2026.9.0" },
    status: "unset",
    severity: 13,
    body: "two edits met",
    attributes: {},
};

/** The gauge point of open notes. */
const open: Entry = {
    kind: "point",
    name: "note.open",
    time: 2_500,
    installation: notes,
    instance: "instance-1",
    source: { name: "@example/notes", version: "2026.9.0" },
    status: "unset",
    metric: "gauge",
    unit: "{note}",
    value: 7,
    attributes: {},
};

/** The latest histogram point over a minute. */
const latency: Entry = {
    kind: "point",
    name: "note.save.duration",
    time: 3_000,
    duration: 60_000_000,
    installation: notes,
    source: { name: "@example/notes", version: "2026.9.0" },
    status: "unset",
    metric: "histogram",
    unit: "ms",
    histogram: {
        count: 3,
        sum: 42.5,
        min: 4,
        max: 30,
        scale: 0,
        zeroCount: 0,
        positive: { offset: 2, counts: [1, 1, 1] },
        negative: { offset: 0, counts: [] },
    },
    attributes: { route: "/notes" },
};

/** A span and a log record inside it, a later warning without a trace, a gauge and a histogram point. */
const entries = [span, saved, conflict, open, latency];

test("roundtrip a segment's entries through Parquet and read a time window and a trace of them", async () => {
    // append out of order and encode
    const segment = new Segment(notes);
    for (const entry of [latency, conflict, open, span, saved]) {
        segment.append(entry);
    }
    const file = (await segment.encode()).buffer;

    // read every entry and only the window around the span
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
        bounds: [1_000, 3_000],
        contents: 1 | (1 << 9) | (1 << 13) | (1 << 25),
    });
});
