import { expect, refusal, test } from "@destack/test";
import { ACCESS_PACKAGE_ID, principal, ANONYMOUS } from "../declare/principal.ts";
import { Caller, CALLER_HEADER, Credential, type GrantClaim } from "./context.ts";
import { schema } from "@destack/schema";

/** The time the caller's credential was verified at. */
const NOW = 1_800_000_000_000;

/** Alice, signed in. */
const alice: Caller = Caller.of(principal.user.reference("universe", "alice"));

/** Alice's session, verified for the access package in one space for a minute. */
const session: Credential = {
    kind: "session",
    id: "session-alice",
    audience: ACCESS_PACKAGE_ID,
    scope: "space-a",
    verifiedAt: NOW,
    expiresAt: NOW + 60_000,
};

test("forward a caller with its credential to a runner and read them back, refusing a malformed one", async () => {
    // forward alice and her session in a request's headers
    const headers = new Headers();
    Caller.forward({ caller: alice, credential: session }, headers);

    // read back the forwarded and absent callers, and refuse a header that is not a JSON caller
    const read = Caller.forwarded(new Request("http://runner.test", { headers }));
    const absent = Caller.forwarded(new Request("http://runner.test"));
    const malformed = ["INVALID_CONTEXT", "the forwarded caller is malformed"];

    expect([read, absent, await refusedHeader("{}"), await refusedHeader("{")]).toEqual([
        { caller: alice, credential: session },
        undefined,
        malformed,
        malformed,
    ]);
});

test("require a credential current for its audience and scope until it lapses", async () => {
    // check alice's session at times, audiences and scopes
    const current = (now: number, scope = "space-a", audience = ACCESS_PACKAGE_ID) =>
        refusal(
            Promise.resolve().then(() => Credential.requireCurrent(session, audience, now, scope)),
        );

    expect({
        live: await current(NOW + 59_999),
        lapsed: await current(NOW + 60_000),
        early: await current(NOW - 5_001),
        scope: await current(NOW, "space-b"),
    }).toEqual({
        live: "done",
        lapsed: ["UNAUTHORIZED", "caller authentication is not current"],
        early: ["UNAUTHORIZED", "caller authentication is not current"],
        scope: ["UNAUTHORIZED", "caller authentication has a different audience or scope"],
    });
});

test("read the key of a credential and the last moment it stays current", () => {
    expect([
        Credential.key(session),
        Credential.clamp(session, NOW + 90_000),
        Credential.clamp(session, NOW),
    ]).toEqual(["session:session-alice", NOW + 59_999, NOW]);
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

test("require actors to act for an identified subject, and a grant to have the actors acting under it", async () => {
    // build a delegate acting for alice, with and without the grant it acts under
    const agent = principal.installation.reference("space-1", "installation-agent");
    const grant: GrantClaim = {
        id: schema.identifier("grant").parse("grant-019f5530-8000-7000-8000-0000000000c1"),
        scope: "user-alice",
        permissions: [],
    };

    // accept an actor under its grant, and refuse a grant without actors or a chain for an anonymous subject
    expect([
        await chained({ ...alice, actors: [agent], grant }),
        await chained({ ...alice, grant }),
        await chained({ ...alice, subject: ANONYMOUS, actors: [agent], grant }),
    ]).toEqual([
        "done",
        ["INVALID_CONTEXT", "a grant needs actors"],
        ["INVALID_CONTEXT", "delegation chain is inconsistent"],
    ]);
});

/** Require a caller's delegation chain, reading its refusal. */
function chained(caller: Caller) {
    return refusal(Promise.resolve().then(() => Caller.requireChain(caller)));
}
