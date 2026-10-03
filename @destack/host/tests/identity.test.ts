import { Scope } from "@destack/sync";
import { principal } from "@destack/access";
import { DirectoryStore } from "@destack/directory";
import { eq } from "@destack/db";
import { PackageId } from "@destack/package";
import { RequestId } from "@destack/service/request";
import {
    Authentication,
    AUTHENTICATION_LIFETIME_MILLISECONDS,
    TokenVerifier,
} from "@destack/service/authentication";
import { ServiceContext } from "@destack/service/server";
import { ResourceContext } from "@destack/resource/context";
import { afterAll, beforeAll, expect, refusal, test } from "@destack/test";
import { found, Identifier } from "@destack/schema";
import { connect } from "@destack/account/client";
import { HostIdentity } from "../src/identity/index.ts";
import { Host, hostKey } from "@destack/account/object";
import { MemoryKeychain } from "../src/keychain/index.ts";
import { GlobalFixture, ACCOUNTS_URL, ids, ISSUER } from "../src/test/index.ts";

/** The refusal of an assertion without an active key behind it. */
const NO_ACTIVE_KEY = "UNAUTHORIZED: host assertion refers to no active key";

/** The package receiving the hosts' requests. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e002");

/** The global tier every scenario enrolls its hosts in. */
let global: GlobalFixture;

beforeAll(async () => {
    global = await GlobalFixture.open();
});

afterAll(async () => {
    await global[Symbol.asyncDispose]();
});

/** Prove a host's key for a token grant at the account service. */
function proveGrant(signer: HostIdentity): Promise<string> {
    return signer.prove(Date.now(), { method: "POST", url: `${ACCOUNTS_URL}/hosts/token` });
}

/** Report a call's outcome as its failure code and message, or as accepted. */
async function outcome(call: Promise<unknown>): Promise<string> {
    const refused = await refusal(call);

    return refused === "done" ? "accepted" : refused.join(": ");
}

test("enroll a host as its account's owner, authenticate it across a key rotation, and refuse it once revoked", async () => {
    const accountId = ids.account;

    // enroll a host under the owner's account and read itself by its token
    const identity = await global.enroll(accountId);
    const itself = () =>
        outcome(global.host(identity).host.get({ accountId, id: identity.hostId }));
    expect(await itself()).toBe("accepted");

    // refuse a stranger enrolling a host into the account, and a proof for another host
    const stranger = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
    const enrolling = {
        accountId,
        requestId: RequestId.create(),
        name: "stray",
        kind: "cloud" as const,
    };
    const other = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
    const { publicKey } = await other.generate();
    expect([
        await outcome(stranger.enroll(global.user(ids.stranger), enrolling)),
        await outcome(
            global.user(ids.owner).host.enroll({
                ...enrolling,
                id: Identifier.create("host"),
                publicKey,
                proof: await other.prove(),
            }),
        ),
    ]).toEqual([
        `NOT_FOUND: no scope ${accountId}`,
        "BAD_REQUEST: device proof is for another device or time",
    ]);

    // number a taken name at enrollment, refuse renaming to a taken one, and refuse a branch separator
    const namesake = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
    const taken = (await global.host(identity).host.get({ accountId, id: identity.hostId })).name;
    const rename = (name: string) =>
        outcome(
            global.host(identity).host.rename({
                accountId,
                id: identity.hostId,
                requestId: RequestId.create(),
                name,
            }),
        );
    expect([
        await outcome(
            namesake.enroll(global.user(ids.owner), {
                ...enrolling,
                requestId: RequestId.create(),
                name: taken,
            }),
        ),
        await rename(`${taken}-2`),
        await rename("studio"),
        await rename("studio--main"),
        await outcome(
            global.user(ids.owner).host.rename({
                accountId,
                id: identity.hostId,
                requestId: RequestId.create(),
                name: "atelier",
            }),
        ),
        await rename("studio"),
    ]).toEqual([
        "accepted",
        "CONFLICT: a record with the same unique key exists",
        "accepted",
        "BAD_REQUEST: invalid input: name: invalid string: must match pattern /^(?!-)(?!.*--)[a-z0-9-]{1,63}(?<!-)$/u",
        "accepted",
        "accepted",
    ]);
    expect((await global.host(identity).host.get({ accountId, id: identity.hostId })).name).toBe(
        "studio",
    );

    // rotate the host's key as the host, and refuse the owner registering a key for it
    await identity.rotate(global.host(identity), accountId);
    expect(await itself()).toBe("accepted");
    const impostor = new HostIdentity(identity.hostId, new MemoryKeychain());
    const { publicKey: foreign, proof } = await impostor.generate();
    expect(
        await outcome(
            global.user(ids.owner).hostKey.create({
                accountId,
                requestId: RequestId.create(),
                parentId: identity.hostId,
                publicKey: foreign,
                proof,
            }),
        ),
    ).toBe("FORBIDDEN: permission denied: rotate");

    // revoke the host as the owner, ending its token at the global tier at once
    await global.user(ids.owner).host.revoke({
        accountId,
        id: identity.hostId,
        requestId: RequestId.create(),
    });
    expect(await itself()).toBe("UNAUTHORIZED: host token's key no longer authenticates");
});

test("grant a host tokens as the host and its region, spending each assertion once, until the host rotates its key, is disabled or is revoked", async () => {
    // enroll a region's host, and keep its first key apart
    const accountId = ids.platform;
    const keys = new MemoryKeychain();
    const identity = new HostIdentity(Identifier.create("host"), keys);
    await identity.enroll(global.user(ids.operator), {
        accountId,
        requestId: RequestId.create(),
        name: "edge",
        kind: "cloud",
        regionId: ids.region,
    });
    const kept = new MemoryKeychain();
    kept.secrets.set(identity.hostId, found(keys.secrets, identity.hostId));
    const first = new HostIdentity(identity.hostId, kept);
    const [key] = await global.database
        .select()
        .from(hostKey.table)
        .where(eq(hostKey.table.parentId, identity.hostId));
    if (key === undefined) {
        throw new TypeError("the enrolled host has no key");
    }

    // grant tokens through the host service with assertions bound to the grant request
    const fetch = (request: Request) => global.accounts.fetch(request);
    const hosts = connect({ url: ACCOUNTS_URL, fetch });
    const grant = async (signer: HostIdentity) =>
        outcome(hosts.hostToken.grant({ assertion: await proveGrant(signer), audience: AUDIENCE }));

    // act as the host and for its region, as the universe's token verifies it
    const { accessToken } = await identity.token(AUDIENCE, ACCOUNTS_URL, fetch);
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: ISSUER,
        audience: AUDIENCE,
        keys: global.keys,
    });
    const verified = await verifier.authenticate(
        new Request("https://edge.test/zones", {
            headers: { authorization: `Bearer ${accessToken}` },
        }),
    );
    const subject = principal.host.reference(accountId, identity.hostId);
    expect([verified.credential, verified.claims.subject, verified.claims.subjects]).toEqual([
        { kind: "host-key", id: key.id },
        subject,
        [subject, principal.region.reference(Scope.universe.id, ids.region)],
    ]);

    // refuse a replayed assertion, and one bound to another request
    const assertion = await proveGrant(identity);
    const other = await identity.prove(Date.now(), {
        method: "POST",
        url: "https://edge.test/token",
    });
    expect([
        await outcome(hosts.hostToken.grant({ assertion, audience: AUDIENCE })),
        await outcome(hosts.hostToken.grant({ assertion, audience: AUDIENCE })),
        await outcome(hosts.hostToken.grant({ assertion: other, audience: AUDIENCE })),
    ]).toEqual([
        "accepted",
        "UNAUTHORIZED: device proof was used before",
        "UNAUTHORIZED: device proof is for another request",
    ]);

    // rotate the key, which revokes the first, and refuse revoking the first again
    await identity.rotate(global.host(identity), accountId);
    expect(
        await outcome(
            global
                .host(identity)
                .hostKey.revoke({ accountId, id: key.id, requestId: RequestId.create() }),
        ),
    ).toBe("CONFLICT: host key is revoked");

    // refuse the first key's assertions and accept the new key's
    expect([await grant(first), await grant(identity)]).toEqual([NO_ACTIVE_KEY, "accepted"]);

    // refuse every grant while the host is disabled, and grant again once enabled
    const status = (method: "disable" | "enable") =>
        global.user(ids.operator).host[method]({
            accountId,
            id: identity.hostId,
            requestId: RequestId.create(),
        });
    await status("disable");
    const disabled = await grant(identity);
    await status("enable");
    expect([disabled, await grant(identity)]).toEqual([
        "UNAUTHORIZED: host is disabled",
        "accepted",
    ]);

    // refuse every grant once the operator revokes the host
    await global.user(ids.operator).host.revoke({
        accountId,
        id: identity.hostId,
        requestId: RequestId.create(),
    });
    expect(await grant(identity)).toBe(NO_ACTIVE_KEY);
});

test("grant a host tokens for the spaces its cell or region serves, and refuse others", async () => {
    // enroll an account's host and a region's host, and place a space in each
    const device = await global.enroll(ids.account);
    const regional = await global.enroll(ids.platform);
    const directory = new DirectoryStore(global.database);
    const own = Identifier.create("space");
    const served = Identifier.create("space");
    const elsewhere = Identifier.create("space");
    await directory.place({ id: own, scope: ids.account, cell: device.hostId, epoch: 1 });
    await directory.place({ id: served, scope: ids.account, cell: ids.region, epoch: 1 });

    // grant tokens in the spaces each host's cell serves
    const hosts = connect({
        url: ACCOUNTS_URL,
        fetch: (request) => global.accounts.fetch(request),
    });
    const grant = async (signer: HostIdentity, spaceId: Identifier<"space">) =>
        outcome(
            hosts.hostToken.grant({
                assertion: await signer.prove(Date.now(), {
                    method: "POST",
                    url: `${ACCOUNTS_URL}/hosts/token`,
                }),
                audience: AUDIENCE,
                spaceId,
            }),
        );
    expect([
        await grant(device, own),
        await grant(regional, served),
        await grant(device, served),
        await grant(regional, elsewhere),
    ]).toEqual([
        "accepted",
        "accepted",
        `FORBIDDEN: ${served} is served elsewhere`,
        `FORBIDDEN: ${elsewhere} is served elsewhere`,
    ]);
});

test("enroll a region's host only for callers who serve the region", async () => {
    // enroll hosts claiming the region as the owner, who lacks the right to serve it
    const enroll = (userId: string, accountId: Identifier<"account">) =>
        outcome(
            new HostIdentity(Identifier.create("host"), new MemoryKeychain()).enroll(
                global.user(userId),
                {
                    accountId,
                    requestId: RequestId.create(),
                    name: "edge",
                    kind: "cloud",
                    regionId: ids.region,
                },
            ),
        );

    // refuse the owner in a rival account and in the platform, and accept the region's operator
    expect([
        await enroll(ids.owner, ids.other),
        await enroll(ids.owner, ids.platform),
        await enroll(ids.operator, ids.platform),
    ]).toEqual([
        "FORBIDDEN: permission denied: serve",
        "FORBIDDEN: permission denied: serve",
        "accepted",
    ]);
});

test("disable, drain and enable a host, refusing its token grants while disabled", async () => {
    const identity = await global.enroll(ids.account);
    const change = (userId: string, name: "disable" | "drain" | "enable") =>
        outcome(
            global.user(userId).host[name]({
                accountId: ids.account,
                id: identity.hostId,
                requestId: RequestId.create(),
            }),
        );
    const hosts = connect({
        url: ACCOUNTS_URL,
        fetch: (request) => global.accounts.fetch(request),
    });
    const verify = async () =>
        outcome(
            hosts.hostToken.grant({
                assertion: await identity.prove(Date.now(), {
                    method: "POST",
                    url: `${ACCOUNTS_URL}/hosts/token`,
                }),
                audience: AUDIENCE,
            }),
        );

    // refuse the host's grants while disabled, grant them while draining and enabled
    const outcomes = [];
    for (const name of ["disable", "drain", "enable"] as const) {
        outcomes.push(await change(ids.owner, name), await verify());
    }
    expect(outcomes).toEqual([
        "accepted",
        "UNAUTHORIZED: host is disabled",
        "accepted",
        "accepted",
        "accepted",
        "accepted",
    ]);

    // refuse a stranger changing the status, and any change once the host is revoked
    const stranger = await change(ids.stranger, "disable");
    await global.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: identity.hostId,
        requestId: RequestId.create(),
    });
    expect([stranger, await change(ids.owner, "enable")]).toEqual([
        `NOT_FOUND: no scope ${ids.account}`,
        "CONFLICT: host is revoked",
    ]);
});

test("refuse host procedures to callers acting as no host, and pass the rest", async () => {
    // keep a host's caller and a user's caller
    const hostSubject = principal.host.reference(
        ids.account,
        "host-01996ab0-0000-7000-8000-00000000000c",
    );
    const userSubject = principal.user.reference(Scope.universe.id, ids.owner);
    const lifetime = {
        audience: AUDIENCE,
        verifiedAt: 1,
        expiresAt: 1 + AUTHENTICATION_LIFETIME_MILLISECONDS,
    };
    const hostCaller = new Authentication({
        ...lifetime,
        credential: { kind: "host-key", id: "key" },
        subject: hostSubject,
        subjects: [hostSubject],
    });
    const userCaller = new Authentication({
        ...lifetime,
        credential: { kind: "session", id: "session" },
        subject: userSubject,
        subjects: [userSubject],
    });

    // decide each caller on a host procedure and on an identity procedure
    const decide = (caller: Authentication, authentication: "host" | "identity") =>
        outcome(
            Host.authorize({
                context: new ServiceContext(new Request("https://hosts.test"), {
                    audience: AUDIENCE,
                    authentication: caller,
                    resources: new ResourceContext(),
                }),
                access: { authentication, permission: null, audit: false },
                path: [],
                input: undefined,
            }),
        );
    expect([
        await decide(hostCaller, "host"),
        await decide(userCaller, "host"),
        await decide(hostCaller, "identity"),
        await decide(userCaller, "identity"),
    ]).toEqual(["accepted", "FORBIDDEN: the procedure requires a host", "accepted", "accepted"]);
});
