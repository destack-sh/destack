import { FOLLOWER_REVISION } from "../test/index.ts";
import { type JsonObject, schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { note, openReplica } from "../test/fixture.ts";
import { definePublication, Subscription } from "./publication.ts";
import { Replica } from "./replica.ts";
import { testWorldOptions } from "@destack/test/world";

test("record a follower's request at its source, keeping its latest revision of a log and taking another epoch's", async () => {
    // record a request, a later revision with other parameters, then an earlier revision arriving late
    const database = await openReplica({
        simulation: new Simulation({ world: testWorldOptions(), seed: 1, start: 1000 }),
    });
    const subscription = {
        name: "notes",
        publication: "queries",
        scope: "inbox",
        target: "inbox",
        parameters: { queries: {} },
        revision: at("first", 1),
    };
    const caller = "user alice";
    const key = { name: "notes", scope: "inbox", target: "inbox", caller };
    const read = async () => {
        const [row] = await Subscription.requests(database, [key]);

        return [row?.parameters, row?.revisionEpoch, row?.revisionSequence];
    };
    await Subscription.request(database, subscription, caller);
    const first = await read();
    const reshaped = { ...subscription, parameters: { queries: { all: { object: "note" } } } };
    await Subscription.request(database, { ...reshaped, revision: at("first", 3) }, caller);
    await Subscription.request(database, { ...subscription, revision: at("first", 2) }, caller);
    const latest = await read();

    // take a restored follower's request of another epoch, whatever its sequence
    await Subscription.request(database, { ...subscription, revision: at("restored", 1) }, caller);
    expect([first, latest, await read()]).toEqual([
        [subscription.parameters, "first", 1],
        [reshaped.parameters, "first", 3],
        [subscription.parameters, "restored", 1],
    ]);
});

test("write a follower's request only when it is new, at a later revision or with other parameters", async () => {
    // record a request, the same one with its parameters' keys reordered, and an earlier revision
    const database = await openReplica({
        simulation: new Simulation({ world: testWorldOptions(), seed: 1, start: 1000 }),
    });
    const subscription = {
        name: "notes",
        publication: "queries",
        scope: "inbox",
        target: "inbox",
        parameters: { folder: "inbox", queries: {} },
        revision: at("first", 2),
    };
    const caller = "user alice";
    const request = (parameters: JsonObject, sequence: number) =>
        Subscription.request(
            database,
            { ...subscription, parameters, revision: at("first", sequence) },
            caller,
        );
    const written = [
        await request({ folder: "inbox", queries: {} }, 2),
        await request({ queries: {}, folder: "inbox" }, 2),
        await request({ folder: "inbox", queries: {} }, 1),

        // take other parameters at the same revision, then the same ones at a later revision
        await request({ folder: "archive", queries: {} }, 2),
        await request({ folder: "archive", queries: {} }, 3),
    ];

    // wait for a position of the source's log, the same one again, an earlier one and a later one
    const waiting = (sequence: number) =>
        Subscription.request(
            database,
            {
                ...subscription,
                parameters: { folder: "archive", queries: {} },
                revision: at("first", 3),
                waiting: at("source", sequence),
            },
            caller,
        );
    const waited = [await waiting(7), await waiting(7), await waiting(6), await waiting(8)];

    // expect writes for the new request, the other parameters, the later revision and each later wait only
    expect({ written, waited }).toEqual({
        written: [true, false, false, true, true],
        waited: [true, false, false, true],
    });
});

test("refuse a subscription whose parameters its publication rejects, reading valid ones as parsed", async () => {
    // declare a publication of notes by a required folder
    const publication = definePublication({
        name: "folder",
        parameters: schema.object({ folder: schema.string().min(1) }),
        audience: "caller",
        replica: ({ name, scope }) => new Replica({ name, scope, tables: [note] }),
    });
    const subscription = {
        name: "folder",
        publication: "folder",
        scope: "inbox",
        target: "inbox",
        revision: FOLLOWER_REVISION,
    };

    // read valid parameters, and refuse missing ones as the caller's mistake
    expect({
        read: publication.read({ ...subscription, parameters: { folder: "inbox" } }).parameters,
        missing: await refusal(() => publication.read({ ...subscription, parameters: {} })),
        replica: await refusal(() =>
            publication.replica({ ...subscription, parameters: { folder: "" } }),
        ),
    }).toEqual({
        read: { folder: "inbox" },
        missing: [
            "BAD_REQUEST",
            "PARAMETERS_INVALID",
            "publication 'folder' is invalid: its parameters do not match: 'folder: invalid input: expected string, received undefined'",
        ],
        replica: [
            "BAD_REQUEST",
            "PARAMETERS_INVALID",
            "publication 'folder' is invalid: its parameters do not match: 'folder: too small: expected string to have >=1 characters'",
        ],
    });
});

/** Build a follower's log position in an epoch. */
function at(epoch: string, sequence: number) {
    return { log: "follower", epoch, sequence };
}
