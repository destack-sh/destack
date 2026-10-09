import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    type OtlpEmitter,
    VISIT_EVENT,
} from "@destack/telemetry/otlp";
import { action, DAY, log, visit, VisitorSalt } from "../src/index.ts";
import { ids, serveObservability } from "./fixture/observability.ts";
import { logsExport, NOTES_MANIFEST } from "./fixture/notes.ts";

/** The browser a page's export came from. */
const BROWSER = { address: "203.0.113.7", userAgent: "Mozilla/5.0 (Macintosh)" };

/** The person signed in to the page. */
const DANA = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000009");

test("keep a page's visits and actions apart from its logs, an anonymous visitor as a hash of the day's salt without its address, a signed-in person's values sealed until forgotten", async () => {
    const { receive, database, events } = await serveObservability();
    const now = Date.now();
    const page: OtlpEmitter = {
        scope: ids.space,
        installation: ids.notes,
        build: NOTES_MANIFEST,
        visitor: BROWSER,
    };

    // visit a note anonymously, then share it signed in, naming an address
    await receive(
        page,
        "logs",
        logsExport([
            {
                time: now,
                name: VISIT_EVENT,
                attributes: {
                    [ANALYTICS_ATTRIBUTES.route]: "/notes/:id",
                    [ANALYTICS_ATTRIBUTES.path]: "/notes/1",
                    "session.id": "session-1",
                },
            },
        ]),
    );
    await receive(
        { ...page, person: DANA },
        "logs",
        logsExport([
            {
                time: now + 1,
                name: ACTION_EVENT,
                attributes: {
                    [ANALYTICS_ATTRIBUTES.action]: "note.shared",
                    "sensitive.email": "erin@example.com",
                    audience: "space",
                },
            },
        ]),
    );

    // read the visits, actions and logs, and the stored rows
    const visits = (await events.query(visit, { scope: ids.space })).events;
    const actions = async () => (await events.query(action, { scope: ids.space })).events;
    const shared = await actions();
    const logs = (await events.query(log, { scope: ids.space })).events;
    const stored = JSON.stringify([
        await database.select().from(visit.table),
        await database.select().from(action.table),
    ]);

    // forget dana
    await events.forget(DANA);
    const forgotten = await actions();

    // keep the visit by its route and session under a hashed visitor, the action sealed under dana, no log, and no address
    const anyVisitor: unknown = expect.stringMatching(/^[0-9a-f]{32}$/u);
    const [visited] = visits;
    const [acted] = shared;
    expect({
        visit: visited === undefined ? undefined : visit.parseKeys(visited.keys),
        action: acted === undefined ? undefined : action.parseKeys(acted.keys),
        personal: shared.map((each) => action.parseData(each.data).personal),
        forgotten: forgotten.map((each) => action.parseData(each.data).personal),
        logs: logs.length,
        isAddressKept: stored.includes(BROWSER.address) || stored.includes(BROWSER.userAgent),
        isEmailKept: stored.includes("erin@example.com"),
    }).toEqual({
        visit: {
            installation: ids.notes,
            build: NOTES_MANIFEST,
            route: "/notes/:id",
            session: "session-1",
            trace: null,
            visitor: anyVisitor,
            person: null,
        },
        action: {
            installation: ids.notes,
            build: NOTES_MANIFEST,
            name: "note.shared",
            route: null,
            session: null,
            trace: null,
            visitor: visited?.keys["visitor"],
            person: DANA,
            attributes: { audience: "space" },
        },
        personal: [{ email: "erin@example.com" }],
        forgotten: [undefined],
        logs: 0,
        isAddressKept: false,
        isEmailKept: false,
    });
});

test("hash a browser under a new salt each day, deleting the day before's so its hashes cannot be made again", async () => {
    const { database } = await serveObservability();
    const origin = { installation: ids.notes, ...BROWSER };
    const day = Date.UTC(2026, 9, 7, 12);

    // hash the browser twice on one day, on the next, and on the first again
    const first = await VisitorSalt.hash(database, day, origin);
    const again = await VisitorSalt.hash(database, day + 1, origin);
    const next = await VisitorSalt.hash(database, day + DAY, origin);
    const returned = await VisitorSalt.hash(database, day, origin);

    // keep one hash a day, never the same across days, and none of a past day's again
    expect({
        isStable: first === again,
        isRotated: first !== next,
        isForgotten: returned !== first,
    }).toEqual({
        isStable: true,
        isRotated: true,
        isForgotten: true,
    });
});
