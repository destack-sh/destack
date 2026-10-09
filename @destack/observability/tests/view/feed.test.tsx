import { expect, test } from "@destack/test";
import type { Event } from "@destack/event";
import { log } from "../../src/index.ts";
import { EventFeed, type EventList } from "../../src/view/feed.ts";
import { eventClient, ids, serveObservability } from "../fixture/observability.ts";
import { logsExport, NOTES_EMITTER } from "../fixture/notes.ts";

/** Read the names of log events. */
function names(events: readonly Event[]): string[] {
    return events.map((event) => log.parseKeys(event.keys).name);
}

test("keep the newest ten pages while live, and pause the tail once an older page loads", async () => {
    const { eventServer, receive } = await serveObservability();
    const reader = eventClient(eventServer, "alice");
    const selection = { scope: ids.space, object: ids.notes, kind: "log" as const };

    // follow the installation's records live, starting from none
    const feed = new EventFeed(reader, selection);
    const following = new AbortController();
    const lists = feed.follow(() => feed.page(), true, following.signal);
    const first = await lists.next();

    // record 1,050 records while live, and follow them until the newest arrives
    const now = Date.now();
    await receive(
        NOTES_EMITTER,
        "logs",
        logsExport(
            Array.from({ length: 1050 }, (_, index) => ({
                time: now + index,
                name: `note.${index}`,
            })),
        ),
    );
    let live: EventList | undefined;
    while (live === undefined || names(live.events)[0] !== "note.1049") {
        const next = await lists.next();
        live = next.done === true ? undefined : next.value;
    }

    // load the older page, which pauses the tail
    live.more?.();
    const older = await lists.next();

    // record one more while a second tail watches, and stop following once it shows there
    const watcher = await reader.tail(selection, { signal: following.signal });
    const pending = lists.next();
    await receive(NOTES_EMITTER, "logs", logsExport([{ time: now + 1050, name: "note.late" }]));
    for await (const event of watcher) {
        if (log.parseKeys(event.keys).name === "note.late") {
            break;
        }
    }
    following.abort();

    // keep the newest 1,000 while live, then the older 50 below them, and nothing after the pause
    expect({
        first: first.done === true ? undefined : first.value.events,
        live: { events: names(live.events), isPartial: live.more !== undefined },
        older: older.done === true ? undefined : names(older.value.events),
        paused: await pending,
    }).toEqual({
        first: [],
        live: {
            events: Array.from({ length: 1000 }, (_, index) => `note.${1049 - index}`),
            isPartial: true,
        },
        older: Array.from({ length: 1050 }, (_, index) => `note.${1049 - index}`),
        paused: { done: true, value: undefined },
    });
});
