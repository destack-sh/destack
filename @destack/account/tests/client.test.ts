import { eq } from "@destack/db";
import { ObjectReference } from "@destack/sync";
import { RequestId } from "@destack/runtime/request";
import { expect, test } from "@destack/test";
import { session } from "../src/object/session.ts";
import { KeyPair, Proof, type ProofRequest, PublicKey } from "@destack/identity";
import { client } from "../src/object/client.ts";
import { key } from "../src/object/key.ts";
import { SESSION_TOKEN_TYPE } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { AccountFixture, outcome } from "../src/test/index.ts";

/** One day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Register a client, prove its keys, attach another session, and end it by revoking the client. */
test("register clients with proven keys and revoke them with the sessions attached to them", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const againSession = await fixture.signIn(owner.id);
    const again = { ...owner, session: againSession, client: fixture.sessionClient(againSession) };
    const signedInOn = async () =>
        (
            await fixture.storage.database
                .select({ clientId: session.table.clientId })
                .from(session.table)
                .where(eq(session.table.token, owner.session))
        ).map((row) => String(row.clientId));
    const [first] = await signedInOn();

    // register a computer, the first session staying on the client it signed in on
    const laptop = await owner.client.client.create({
        userId: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "laptop",
        kind: "native",
        platform: "macOS",
    });
    expect(laptop).toEqual({
        id: laptop.id,
        createdAt: laptop.createdAt,
        ...ObjectReference.columns("createdBy", owner.subject),
        ...ObjectReference.columns("updatedBy", owner.subject),
        updatedAt: laptop.createdAt,
        revision: 1,
        tags: {},
        scope: owner.id,
        name: "laptop",
        kind: "native",
        platform: "macOS",
        platformVersion: null,
        architecture: null,
        model: null,
        machineId: null,
        version: null,
        lastActiveAt: laptop.createdAt,
        revokedAt: null,
    });
    const attached = async () =>
        (
            await fixture.storage.database
                .select()
                .from(session.table)
                .where(eq(session.table.userId, owner.id))
        )
            .map((row) => String(row.clientId))
            .toSorted((left, right) => left.localeCompare(right));
    const sessionsOn = async (id: string) =>
        (await attached()).filter((attachedTo) => attachedTo === id).length;
    expect([await signedInOn(), await sessionsOn(laptop.id)]).toEqual([[first], 0]);

    // register a browser profile
    const browser = await owner.client.client.create({
        userId: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "Firefox",
        kind: "browser",
    });
    expect([browser.kind, await sessionsOn(browser.id)]).toEqual(["browser", 0]);

    // register a key the client proves it has
    const pair = (await KeyPair.generate()).pair;
    const parent = { ...client.typeReference, id: laptop.id };
    const register = async (publicKey: PublicKey, proof: string) =>
        owner.client.key.create({
            scope: owner.id,
            requestId: RequestId.create(Date.now()),
            parent,
            publicKey,
            proof,
        });
    const registered = await register(pair.publicKey, await prove(pair, laptop.id));
    expect(registered).toEqual({
        id: registered.id,
        createdAt: registered.createdAt,
        ...ObjectReference.columns("createdBy", owner.subject),
        ...ObjectReference.columns("updatedBy", owner.subject),
        updatedAt: registered.createdAt,
        revision: 1,
        tags: {},
        scope: owner.id,
        parentPackageId: client.typeReference.packageId,
        parentType: "client",
        parentId: laptop.id,
        publicKey: pair.publicKey,
        thumbprint: await PublicKey.thumbprint(pair.publicKey),
        verifiedAt: registered.createdAt,
        expiresAt: registered.createdAt + 365 * DAY,
        revokedAt: null,
        suspendedAt: null,
    });
    expect(await outcome(register(pair.publicKey, await prove(pair, laptop.id)))).toEqual(
        "CONFLICT: another key has the same thumbprint",
    );

    // refuse proofs for another client, time or key
    const other = (await KeyPair.generate()).pair;
    const refusals = await Promise.all(
        [
            prove(other, "client-elsewhere"),
            prove(other, laptop.id, Date.now() - 120_000),
            prove(pair, laptop.id),
            prove(other, laptop.id).then((proof) => `${proof.slice(0, -4)}AAAA`),
            prove(other, laptop.id, Date.now(), {
                method: "GET",
                url: "https://account.test/user",
            }),
            Promise.resolve("not-a-proof"),
        ].map(async (proof) => outcome(register(other.publicKey, await proof))),
    );
    expect(refusals).toEqual([
        "BAD_REQUEST: key proof is for another subject or time",
        "BAD_REQUEST: key proof is for another subject or time",
        "BAD_REQUEST: key proof signature is invalid",
        "BAD_REQUEST: key proof signature is invalid",
        "BAD_REQUEST: key proof is for another request",
        "BAD_REQUEST: key proof is no compact JWS",
    ]);

    // attach the second session with a proof by the registered key
    const attach = async (proof: string) =>
        again.client.client.attach({
            userId: owner.id,
            id: laptop.id,
            requestId: RequestId.create(Date.now()),
            proof,
        });
    expect(await outcome(attach(await prove(other, laptop.id)))).toBe(
        "FORBIDDEN: key proof refers to no active key of the client",
    );
    const proof = await prove(pair, laptop.id);
    await attach(proof);
    expect(await sessionsOn(laptop.id)).toBe(1);

    // refuse the same proof a second time
    expect(await outcome(attach(proof))).toBe("BAD_REQUEST: key proof was used before");

    // hide the client from another user
    const stranger = await fixture.person("stranger@example.com");
    expect(await outcome(stranger.client.client.get({ userId: owner.id, id: laptop.id }))).toBe(
        `NOT_FOUND: no client ${laptop.id}`,
    );

    // keep the browser's push subscription
    const push = {
        url: "https://push.example.com/subscription",
        keys: { p256dh: "BNcR", auth: "tBHI" },
    };
    const kept = async () =>
        (
            await fixture.storage.database
                .select({ push: client.table.push })
                .from(client.table)
                .where(eq(client.table.id, browser.id))
        ).map((row) => row.push);
    const subscribed = await owner.client.client.subscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(Date.now()),
        push,
    });
    const refusedPush = await outcome(
        owner.client.client.subscribe({
            userId: owner.id,
            id: laptop.id,
            requestId: RequestId.create(Date.now()),
            push,
        }),
    );
    expect(await kept()).toEqual([push]);
    await owner.client.client.unsubscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(Date.now()),
    });
    expect([subscribed.revision, refusedPush, await kept()]).toEqual([
        browser.revision + 1,
        "BAD_REQUEST: only browsers subscribe to push",
        [null],
    ]);

    // revoke the computer, its key and the session attached to it
    const revoked = await owner.client.client.revoke({
        userId: owner.id,
        id: laptop.id,
        requestId: RequestId.create(Date.now()),
    });
    expect(revoked.revokedAt).toBe(revoked.updatedAt);
    const [ended] = await fixture.storage.database
        .select({ revokedAt: key.table.revokedAt })
        .from(key.table)
        .where(eq(key.table.id, registered.id));
    expect(ended?.revokedAt).toBe(revoked.revokedAt);
    expect(
        await outcome(
            fixture.exchangeToken(again.session, SESSION_TOKEN_TYPE, {
                audience: accountPackage.id,
            }),
        ),
    ).toBe("BAD_REQUEST: invalid_grant: session is no longer authorized");
});

/** Sign a proof for a client or machine by a key pair at a time, bound to a request if given. */
function prove(
    pair: KeyPair,
    subject: string,
    now = Date.now(),
    request?: ProofRequest,
): Promise<string> {
    return Proof.sign(pair.privateKey, pair.publicKey, subject, now, request);
}
