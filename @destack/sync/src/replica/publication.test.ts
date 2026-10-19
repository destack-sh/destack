import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import type { Call } from "../call/call.ts";
import type { Page } from "../query/index.ts";
import { Uplink } from "./publication.ts";

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
