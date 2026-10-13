import { expect, refusal, test } from "@destack/test";
import { ACCESS_PACKAGE_ID, principal } from "../declare/principal.ts";
import { Caller, CALLER_HEADER, type CallerGrant } from "./context.ts";
import { schema } from "@destack/schema";

/** The time the caller's credential was verified at. */
const NOW = 1_800_000_000_000;

/** Alice, verified for the access package in one space for a minute. */
const alice: Caller = {
    subject: principal.user.reference("universe", "alice"),
    subjects: [principal.user.reference("universe", "alice")],
    credential: {
        kind: "session",
        id: "session-alice",
        audience: ACCESS_PACKAGE_ID,
        scope: "space-a",
        verifiedAt: NOW,
        expiresAt: NOW + 60_000,
    },
};

test("forward a caller to a runner and read it back, refusing a malformed one", async () => {
    // forward alice in a request's headers
    const headers = new Headers();
    Caller.forward(alice, headers);

    // read back the forwarded and absent callers, and refuse a header that is no caller or no JSON
    const read = Caller.forwarded(new Request("http://runner.test", { headers }));
    const absent = Caller.forwarded(new Request("http://runner.test"));
    const malformed = ["INVALID_CONTEXT", "the forwarded caller is malformed"];

    expect([read, absent, await refusedHeader("{}"), await refusedHeader("{")]).toEqual([
        alice,
        undefined,
        malformed,
        malformed,
    ]);
});

test("require a caller current for its audience and scope until its credential lapses, and a principal running its own code always", async () => {
    // check alice at times, audiences and scopes
    const current = (now: number, scope = "space-a", audience = ACCESS_PACKAGE_ID) =>
        refusal(Promise.resolve().then(() => Caller.requireCurrent(alice, audience, now, scope)));
    const own = Caller.of(principal.user.reference("universe", "bob"), NOW);

    expect({
        live: await current(NOW + 59_999),
        lapsed: await current(NOW + 60_000),
        early: await current(NOW - 5_001),
        scope: await current(NOW, "space-b"),
        own: await refusal(
            Promise.resolve().then(() => Caller.requireCurrent(own, ACCESS_PACKAGE_ID, NOW + 1e9)),
        ),
    }).toEqual({
        live: "done",
        lapsed: ["UNAUTHORIZED", "caller authentication is not current"],
        early: ["UNAUTHORIZED", "caller authentication is not current"],
        scope: ["UNAUTHORIZED", "caller authentication has a different audience or scope"],
        own: "done",
    });
});

test("read the key of a caller's credential and the last moment it stays current", () => {
    expect([Caller.credentialKey(alice), Caller.clamp(alice, NOW + 90_000)]).toEqual([
        "session:session-alice",
        NOW + 59_999,
    ]);
});

/** Read the refusal of a caller forwarded in a header. */
function refusedHeader(header: string) {
    return refusal(
        Promise.resolve().then(() =>
            Caller.forwarded(
                new Request("http://runner.test", { headers: { [CALLER_HEADER]: header } }),
            ),
        ),
    );
}

test("require delegates to act for a verified subject, and a grant to have the delegates acting under it", async () => {
    // build a delegate acting for alice, with and without the grant it acts under
    const agent = principal.installation.reference("space-1", "installation-agent");
    const grant: CallerGrant = {
        id: schema.identifier("grant").parse("grant-019f5530-8000-7000-8000-0000000000c1"),
        scope: "user-alice",
        permissions: [],
    };

    // accept a delegate under its grant, and refuse a grant without delegates or a chain for an unverified subject
    expect([
        await chained({ ...alice, delegates: [agent], grant }),
        await chained({ ...alice, grant }),
        await chained({ ...alice, subjects: [], delegates: [agent], grant }),
    ]).toEqual([
        "done",
        ["INVALID_CONTEXT", "a grant needs the delegates acting under it"],
        ["INVALID_CONTEXT", "delegation chain is inconsistent"],
    ]);
});

/** Require a caller's delegation chain, reading its refusal. */
function chained(caller: Caller) {
    return refusal(Promise.resolve().then(() => Caller.requireChain(caller)));
}
