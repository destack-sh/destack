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
    const malformed = [
        "BAD_REQUEST",
        "FORWARDED_CALLER_INVALID",
        "the forwarded caller is malformed",
    ];

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
        lapsed: ["UNAUTHORIZED", "AUTHENTICATION_EXPIRED", "caller authentication is not current"],
        early: ["UNAUTHORIZED", "AUTHENTICATION_EXPIRED", "caller authentication is not current"],
        scope: [
            "UNAUTHORIZED",
            "AUDIENCE_MISMATCH",
            "caller authentication has a different audience or scope",
        ],
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

test("require a grant per actor or none, and actors to act for an identified subject", async () => {
    // build an agent and its sub-agent acting for alice, with and without their grants
    const agent = principal.installation.reference("space-1", "installation-agent");
    const subagent = principal.installation.reference("space-1", "installation-subagent");
    const grant: GrantClaim = {
        id: schema.identifier("grant").parse("grant-019f5530-8000-7000-8000-0000000000c1"),
        scope: "user-alice",
    };

    // accept a grant per actor and an impersonation, refuse a chain short of grants or for no one
    expect([
        await chained({ ...alice, actors: [agent], grants: [grant] }),
        await chained({ ...alice, actors: [agent, subagent], grants: [grant, grant] }),
        await chained({ ...alice, actors: [agent] }),
        await chained({ ...alice, grants: [grant] }),
        await chained({ ...alice, actors: [agent, subagent], grants: [grant] }),
        await chained({ ...alice, subject: ANONYMOUS, actors: [agent], grants: [grant] }),
    ]).toEqual([
        "done",
        "done",
        "done",
        ["UNAUTHORIZED", "DELEGATION_INVALID", "a delegation chain needs a grant per actor"],
        ["UNAUTHORIZED", "DELEGATION_INVALID", "a delegation chain needs a grant per actor"],
        ["UNAUTHORIZED", "DELEGATION_INVALID", "delegation chain is inconsistent"],
    ]);
});

test("link each grant of a chain to its delegator, the actor before it, and its delegate", () => {
    // chain alice's agent and its sub-agent
    const agent = principal.installation.reference("space-1", "installation-agent");
    const subagent = principal.installation.reference("space-1", "installation-subagent");
    const toAgent = claimOf("grant-019f5530-8000-7000-8000-0000000000c2");
    const toSubagent = claimOf("grant-019f5530-8000-7000-8000-0000000000c3");

    // link alice to the agent, and the agent to the sub-agent
    expect(
        Caller.links({ ...alice, actors: [agent, subagent], grants: [toAgent, toSubagent] }).map(
            (link) => [link.delegator.id, link.delegate.id, link.grant.id],
        ),
    ).toEqual([
        [alice.subject.id, agent.id, toAgent.id],
        [agent.id, subagent.id, toSubagent.id],
    ]);
});

/** Claim a grant stored in the agents' space. */
function claimOf(id: string): GrantClaim {
    return { id: schema.identifier("grant").parse(id), scope: "space-1" };
}

/** Require a caller's delegation chain, reading its refusal. */
function chained(caller: Caller) {
    return refusal(Promise.resolve().then(() => Caller.requireChain(caller)));
}
