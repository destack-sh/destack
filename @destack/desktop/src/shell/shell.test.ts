import { expect, refusal, test } from "@destack/test";
import { Shell } from "./shell.ts";

/** A window's identity the events name. */
const WINDOW = "window-01996ab0-0000-7000-8000-000000000001";

/** Read every event a shell reads from chunks of its pipe. */
async function read(chunks: readonly string[]): Promise<unknown[]> {
    // send each chunk, then close the pipe
    const encoder = new TextEncoder();
    const input = new ReadableStream<Uint8Array>({
        start: (controller) => {
            for (const chunk of chunks) {
                controller.enqueue(encoder.encode(chunk));
            }
            controller.close();
        },
    });

    // read the events until the pipe closes
    const shell = new Shell(async () => {}, input);
    const events: unknown[] = [];
    for await (const event of shell.events()) {
        events.push(event);
    }

    return events;
}

test("read events split across chunks and lines, ending a line with or without a carriage return", async () => {
    // split one event across two chunks, and send two in one chunk
    const events = await read([
        `{"action":"window.focused","id":"${WINDOW}"}\r\n{"action":"shortcut.pre`,
        `ssed","id":"palette"}\n{"action":"window.closed","id":"${WINDOW}"}\n`,
    ]);

    // expect the three events in order
    expect(events).toEqual([
        { action: "window.focused", id: WINDOW },
        { action: "shortcut.pressed", id: "palette" },
        { action: "window.closed", id: WINDOW },
    ]);
});

test("refuse a line that is no JSON, a line that is no event, and a pipe closed within a line", async () => {
    expect([
        await refusal(read(["not json\n"])),
        await refusal(read(['{"action":"window.flown"}\n'])),
        await refusal(read(['{"action":"window.closed"'])),
    ]).toEqual([
        ["BAD_REQUEST", "EVENT_INVALID", "the shell sent no JSON: not json"],
        ["BAD_REQUEST", "EVENT_INVALID", 'the shell sent no event: {"action":"window.flown"}'],
        [
            "BAD_REQUEST",
            "EVENT_INVALID",
            'the shell closed its pipe within a line: {"action":"window.closed"',
        ],
    ]);
});
