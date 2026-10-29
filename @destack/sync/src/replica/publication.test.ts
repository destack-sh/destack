import { FOLLOWER_REVISION } from "../test/index.ts";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import type { Call } from "../call/call.ts";
import type { Page } from "../query/index.ts";
import { Simulation } from "@destack/environment/simulation";
import { note, openReplica } from "../test/fixture.ts";
import { definePublication, Subscription, Uplink } from "./publication.ts";
import { Replica } from "./replica.ts";

/** The package of the notes the scenarios send. */
const PACKAGE_ID = schema
    .identifier("package")
    .parse("package-01996ab0-0000-7000-8000-000000000001");

/** Build a call of a note method. */
function noteCall(method: string): Call {
    return {
        type: { packageId: PACKAGE_ID, type: "note" },
        method,
        release: "2026.10.0",
        input: { id: "a" },
    };
}

test("stream through a client's replica procedures as the follower, and send a forwarded change and query as their signer", async () => {
    // call a fake client, recording whom each call signs as
    const signers: string[] = [];
    const uplink = Uplink.of((signer) => {
        signers.push(signer === undefined ? "follower" : signer.id);

        return {
            stream: async () => (async function* (): AsyncIterable<Page> {})(),
            request: async () => ({}),
            receive: async () => ({ results: [{ id: "a" }] }),
            call: async (input: { readonly scope: string }) => ({ answered: input.scope }),
        };
    });

    // stream a replica, send a change an installation signs, and a query the space signs
    const pages = await Array.fromAsync(
        uplink.stream(
            {
                name: "notes",
                scope: "inbox",
                target: "inbox",
                publication: "notes",
                parameters: {},
                revision: FOLLOWER_REVISION,
            },
            AbortSignal.timeout(1000),
        ),
    );
    const installation = { packageId: PACKAGE_ID, type: "installation", scope: "inbox" };
    const results = await uplink.receive(
        { id: "019f5530-8000-7000-8000-000000000001", calls: [noteCall("create")] },
        { ...installation, id: "installation-a" },
    );
    const answer = await uplink.call(noteCall("summary"), "inbox", {
        packageId: PACKAGE_ID,
        type: "space",
        scope: "universe",
        id: "space-a",
    });

    // expect each signer in order, the change's results and the query's answer
    expect([pages, signers, results, answer]).toEqual([
        [],
        ["follower", "installation-a", "space-a"],
        [{ id: "a" }],
        { answered: "inbox" },
    ]);
});

test("record a follower's request at its source, keeping its latest revision of a log and taking another epoch's", async () => {
    // record a request, a later revision with other parameters, then an earlier revision arriving late
    const database = await openReplica({ simulation: new Simulation({ seed: 1, start: 1000 }) });
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
            "invalid parameters of publication folder: folder: invalid input: expected string, received undefined",
        ],
        replica: [
            "BAD_REQUEST",
            "PARAMETERS_INVALID",
            "invalid parameters of publication folder: folder: too small: expected string to have >=1 characters",
        ],
    });
});

/** Build a follower's log position in an epoch. */
function at(epoch: string, sequence: number) {
    return { log: "follower", epoch, sequence };
}
