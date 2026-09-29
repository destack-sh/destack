import { v7 } from "uuid";
import { Scope } from "@destack/sync";
import { principal } from "@destack/access";
import { DirectoryDatabase } from "@destack/directory";
import { eq } from "@destack/db";
import { PackageId } from "@destack/package";
import { RequestId } from "@destack/service/request";
import { Caller, CALLER_LIFETIME_MILLISECONDS } from "@destack/service/authentication";
import type { ProcedureCall, ServiceContext } from "@destack/service/server";
import { afterAll, beforeAll, expect, test } from "@destack/test";
import { identifier } from "@destack/schema";
import { HostCaller, HostIdentity } from "../src/identity/index.ts";
import { hostKey } from "../src/object/index.ts";
import { MemoryKeychain } from "../src/keychain/index.ts";
import { GlobalFixture, ids } from "../src/test/index.ts";

/** The refusal of a proof without an active key behind it. */
const NO_ACTIVE_KEY = "UNAUTHORIZED: host proof refers to no active key";

/** The package receiving the hosts' requests. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e002");

/** The global tier every scenario enrolls its own hosts in. */
let global: GlobalFixture;

beforeAll(async () => {
    global = await GlobalFixture.open();
});

afterAll(async () => {
    await global[Symbol.asyncDispose]();
});

/** Report a call's outcome as its failure code and message, or as accepted. */
function outcome(call: Promise<unknown>): Promise<string> {
    return call.then(
        () => "accepted",
        (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
    );
}

test("enroll a host as its account's owner, authenticate its proofs across a key rotation, and refuse them once revoked", async () => {
    const accountId = ids.account;

    // enroll a host under the owner's account and read itself by its proofs
    const identity = await global.enroll(accountId);
    const itself = () =>
        outcome(global.host(identity).host.get({ accountId, id: identity.hostId as never }));
    expect(await itself()).toBe("accepted");

    // refuse a stranger enrolling a host into the account, and a proof for another host
    const stranger = new HostIdentity(`host-${v7()}`, new MemoryKeychain());
    const enrolling = {
        accountId,
        requestId: RequestId.create(),
        name: "stray",
        kind: "cloud" as const,
    };
    const other = new HostIdentity(`host-${v7()}`, new MemoryKeychain());
    const { publicKey } = await other.generate();
    expect([
        await outcome(stranger.enroll(global.user(ids.stranger), enrolling)),
        await outcome(
            global.user(ids.owner).host.enroll({
                ...enrolling,
                id: identifier("host").parse(`host-${v7()}`),
                publicKey,
                proof: await other.prove(),
            }),
        ),
    ]).toEqual([
        `NOT_FOUND: no scope ${accountId}`,
        "BAD_REQUEST: device proof is for another device or time",
    ]);

    // number a taken name at enrollment, refuse renaming to a taken one, and refuse a branch separator
    const namesake = new HostIdentity(`host-${v7()}`, new MemoryKeychain());
    const taken = (
        await global.host(identity).host.get({ accountId, id: identity.hostId as never })
    ).name;
    const rename = (name: string) =>
        outcome(
            global.host(identity).host.rename({
                accountId,
                id: identity.hostId as never,
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
                id: identity.hostId as never,
                requestId: RequestId.create(),
                name: "atelier",
            }),
        ),
        await rename("studio"),
    ]).toEqual([
        "accepted",
        "CONFLICT: a record with the same unique key exists",
        "accepted",
        "BAD_REQUEST: invalid input: name: invalid string: must match pattern /^(?!-)(?!.*--)[a-z0-9-]{1,63}(?<!-)$/",
        "accepted",
        "accepted",
    ]);
    expect(
        (await global.host(identity).host.get({ accountId, id: identity.hostId as never })).name,
    ).toBe("studio");

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
                parentId: identity.hostId as never,
                publicKey: foreign,
                proof,
            }),
        ),
    ).toBe("FORBIDDEN: permission denied: rotate");

    // revoke the host as the owner, ending its proofs
    await global.user(ids.owner).host.revoke({
        accountId,
        id: identity.hostId as never,
        requestId: RequestId.create(),
    });
    expect(await itself()).toBe(NO_ACTIVE_KEY);
});

test("verify a host's proofs as the host and its region, and as a peer through its host keys until the host revokes a key", async () => {
    // enroll a host for the platform's region and keep its first key apart
    const accountId = ids.platform;
    const keys = new MemoryKeychain();
    const identity = new HostIdentity(`host-${v7()}`, keys);
    await identity.enroll(global.user(ids.operator), {
        accountId,
        requestId: RequestId.create(),
        name: "edge",
        kind: "cloud",
        region: ids.region,
    });
    const kept = new MemoryKeychain();
    kept.secrets.set(identity.hostId, keys.secrets.get(identity.hostId)!);
    const first = new HostIdentity(identity.hostId, kept);
    const [key] = await global.database
        .select()
        .from(hostKey.table)
        .where(eq(hostKey.table.parentId, identifier("host").parse(identity.hostId)));

    // publish where the host answers, and enroll a peer verifying it
    const directory = new DirectoryDatabase(global.database);
    await directory.publish(identity.hostId, accountId, "https://edge.test");
    const verifier = global.host(await global.enroll(ids.other));

    // act as the host and for its region in the global tier, and as the host alone to a peer
    const request = (headers: HeadersInit = {}) =>
        new Request("https://edge.test/zones", { method: "POST", headers });
    const now = Date.now();
    const verified = await HostCaller.authenticate(
        await identity.sign(request(), now),
        global.database,
        AUDIENCE,
        now,
    );
    const peer = await HostCaller.peer(
        await identity.sign(request(), now),
        verifier,
        directory,
        global.database,
        AUDIENCE,
        now,
    );
    const subject = principal.host.reference(accountId, identity.hostId);
    const verification = {
        audience: AUDIENCE,
        verifiedAt: now,
        expiresAt: now + CALLER_LIFETIME_MILLISECONDS,
        subject,
    };
    expect([verified.authentication, peer.authentication]).toEqual([
        {
            ...verification,
            credential: { kind: "host-key", id: key!.id, hostId: identity.hostId },
            subjects: [subject, principal.region.reference(Scope.universe.id, ids.region)],
        },
        {
            ...verification,
            credential: { kind: "host-key", id: key!.id, hostId: identity.hostId },
            subjects: [subject],
        },
    ]);

    // refuse a replayed proof in the global tier and at peers, and a proof beside cookies
    const verifyGlobal = (signed: Request) =>
        outcome(HostCaller.authenticate(signed, global.database, AUDIENCE));
    const verifyPeer = (signed: Request) =>
        outcome(HostCaller.peer(signed, verifier, directory, global.database, AUDIENCE));
    const [toGlobal, toPeer] = [await identity.sign(request()), await identity.sign(request())];
    expect([
        [await verifyGlobal(toGlobal.clone()), await verifyGlobal(toGlobal)],
        [await verifyPeer(toPeer.clone()), await verifyPeer(toPeer)],
        await verifyPeer(await identity.sign(request({ cookie: "session=1" }))),
    ]).toEqual([
        ["accepted", "UNAUTHORIZED: device proof was used before"],
        ["accepted", "UNAUTHORIZED: device proof was used before"],
        "UNAUTHORIZED: invalid host proof",
    ]);

    // rotate the key, which revokes the first, and refuse revoking the first again
    await identity.rotate(global.host(identity), accountId);
    expect(
        await outcome(
            global
                .host(identity)
                .hostKey.revoke({ accountId, id: key!.id, requestId: RequestId.create() }),
        ),
    ).toBe("CONFLICT: host key is revoked");

    // refuse the first key's proofs in the global tier and at peers, and accept the new key's
    const verify = async (signer: HostIdentity) => [
        await verifyGlobal(await signer.sign(request())),
        await verifyPeer(await signer.sign(request())),
    ];
    expect([await verify(first), await verify(identity)]).toEqual([
        [NO_ACTIVE_KEY, NO_ACTIVE_KEY],
        ["accepted", "accepted"],
    ]);

    // refuse every proof in the global tier and at peers while the host is disabled, and accept them again once enabled
    const status = (method: "disable" | "enable") =>
        global.user(ids.operator).host[method]({
            accountId,
            id: identity.hostId as never,
            requestId: RequestId.create(),
        });
    await status("disable");
    const disabled = await verify(identity);
    await status("enable");
    expect([disabled, await verify(identity)]).toEqual([
        ["UNAUTHORIZED: host is disabled", NO_ACTIVE_KEY],
        ["accepted", "accepted"],
    ]);

    // refuse every proof in the global tier and at peers once the operator revokes the host
    await global.user(ids.operator).host.revoke({
        accountId,
        id: identity.hostId as never,
        requestId: RequestId.create(),
    });
    expect(await verify(identity)).toEqual([NO_ACTIVE_KEY, NO_ACTIVE_KEY]);
});

test("enroll a region's host only for callers who serve the region", async () => {
    // enroll hosts claiming the region as the owner, who lacks the right to serve it
    const enroll = (userId: string, accountId: string) =>
        outcome(
            new HostIdentity(`host-${v7()}`, new MemoryKeychain()).enroll(global.user(userId), {
                accountId: identifier("account").parse(accountId),
                requestId: RequestId.create(),
                name: "edge",
                kind: "cloud",
                region: ids.region,
            }),
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

test("disable, drain and enable a host, refusing its proofs while disabled", async () => {
    const identity = await global.enroll(ids.account);
    const change = (userId: string, name: "disable" | "drain" | "enable") =>
        outcome(
            global.user(userId).host[name]({
                accountId: ids.account,
                id: identity.hostId as never,
                requestId: RequestId.create(),
            }),
        );
    const verify = async () =>
        outcome(
            HostCaller.authenticate(
                await identity.sign(new Request("https://edge.test/zones")),
                global.database,
                AUDIENCE,
            ),
        );

    // refuse the host's proofs while disabled, accept them while draining and enabled
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
        id: identity.hostId as never,
        requestId: RequestId.create(),
    });
    expect([stranger, await change(ids.owner, "enable")]).toEqual([
        `NOT_FOUND: no scope ${ids.account}`,
        "CONFLICT: host is revoked",
    ]);
});

test("refuse host procedures to callers presenting no host key, and pass the rest", async () => {
    // hold a host's caller and a user's caller
    const hostSubject = principal.host.reference(
        ids.account,
        "host-01996ab0-0000-7000-8000-00000000000c",
    );
    const userSubject = principal.user.reference(Scope.universe.id, ids.owner);
    const lifetime = {
        audience: AUDIENCE,
        verifiedAt: 1,
        expiresAt: 1 + CALLER_LIFETIME_MILLISECONDS,
    };
    const hostCaller = new HostCaller({
        ...lifetime,
        credential: { kind: "host-key", id: "key", hostId: hostSubject.id },
        subject: hostSubject,
        subjects: [hostSubject],
    });
    const userCaller = new Caller({
        ...lifetime,
        credential: { kind: "session", id: "session" },
        subject: userSubject,
        subjects: [userSubject],
    });

    // decide each caller on a host procedure and on an identity procedure
    const decide = (caller: Caller, authentication: "host" | "identity") =>
        outcome(
            HostCaller.authorize({
                context: { caller },
                access: { authentication, permission: null, audit: false },
            } as unknown as ProcedureCall<ServiceContext>),
        );
    expect([
        await decide(hostCaller, "host"),
        await decide(userCaller, "host"),
        await decide(hostCaller, "identity"),
        await decide(userCaller, "identity"),
    ]).toEqual([
        "accepted",
        "FORBIDDEN: the procedure requires a host key",
        "accepted",
        "accepted",
    ]);
});
