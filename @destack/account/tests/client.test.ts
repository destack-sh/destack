import type { Environment } from "@destack/environment";
import { eq } from "@destack/db";
import { ObjectReference, RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { session } from "../src/object/session.ts";
import { KeyPair, Proof, PublicKey } from "@destack/identity";
import { client } from "../src/object/client.ts";
import { key } from "../src/object/key.ts";
import accountPackage from "../src/package.ts";
import { TestAccountStore, STRANGER_EMAIL } from "../src/test/index.ts";

/** The browser's public key of RFC 8291's example subscription. */
const RFC_8291_KEY =
    "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";

/** The authentication secret of RFC 8291's example subscription. */
const RFC_8291_AUTH = "BTBZMqHH6r4Tts7J_aSIgg";

/** One day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Sort client identifiers as the sessions' listing does. */
function sortIds(ids: readonly string[]): string[] {
    return ids.toSorted((left, right) => left.localeCompare(right));
}

/** Sign a proof for a client or machine by a key pair at a time, in a world. */
function prove(
    pair: KeyPair,
    subject: string,
    now: number,
    world: Pick<Environment, "crypto" | "random">,
): Promise<string> {
    return Proof.sign(pair.privateKey, pair.publicKey, subject, now, world);
}

test("register clients with proven keys and revoke them with the sessions linked to them", async () => {
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn("person@example.com");
    const again = await fixture.signIn("person@example.com");
    const now = fixture.clock.now();

    // register a computer, the first session staying on the client it signed in on
    const laptop = await owner.client(accountPackage).client.create({
        userId: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        name: "laptop",
        kind: "native",
        platform: "macOS",
    });
    expect(laptop).toEqual({
        id: laptop.id,
        createdAt: laptop.createdAt,
        ...ObjectReference.columns("createdBy", owner.subject),
        ...ObjectReference.columns("updatedBy", owner.subject),
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
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
    const linked = async () =>
        (
            await fixture.storage.database
                .select({ clientId: session.table.clientId })
                .from(session.table)
                .where(eq(session.table.userId, owner.id))
        )
            .map((row) => String(row.clientId))
            .toSorted((left, right) => left.localeCompare(right));
    expect(await linked()).toEqual(sortIds([owner.clientId, again.clientId]));

    // register a browser profile
    const browser = await owner.client(accountPackage).client.create({
        userId: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        name: "Firefox",
        kind: "browser",
    });
    expect(await linked()).toEqual(sortIds([owner.clientId, again.clientId]));

    // register a key the client proves it has
    const pair = (await KeyPair.generate(fixture.environment.crypto)).pair;
    const parent = { ...client.typeReference, id: laptop.id };
    const register = async (publicKey: PublicKey, proof: string) =>
        owner.client(accountPackage).key.create({
            scope: owner.id,
            requestId: RequestId.create(fixture.store.environment),
            parent,
            publicKey,
            proof,
        });
    const registered = await register(
        pair.publicKey,
        await prove(pair, laptop.id, fixture.store.clock.now(), fixture.environment),
    );
    expect(registered).toEqual({
        id: registered.id,
        createdAt: registered.createdAt,
        ...ObjectReference.columns("createdBy", owner.subject),
        ...ObjectReference.columns("updatedBy", owner.subject),
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
        updatedAt: registered.createdAt,
        revision: 1,
        tags: {},
        scope: owner.id,
        parentPackageId: client.typeReference.packageId,
        parentType: "client",
        parentId: laptop.id,
        publicKey: pair.publicKey,
        thumbprint: await PublicKey.thumbprint(pair.publicKey, fixture.environment.crypto),
        verifiedAt: registered.createdAt,
        expiresAt: registered.createdAt + 365 * DAY,
        revokedAt: null,
        suspendedAt: null,
    });
    expect(
        await refusal(
            register(
                pair.publicKey,
                await prove(pair, laptop.id, fixture.store.clock.now(), fixture.environment),
            ),
        ),
    ).toEqual(["CONFLICT", "DUPLICATE", "another key has the same thumbprint"]);

    // refuse a proof for another client
    const other = (await KeyPair.generate(fixture.environment.crypto)).pair;
    const elsewhere = await prove(
        other,
        "client-elsewhere",
        fixture.store.clock.now(),
        fixture.environment,
    );
    expect(await refusal(register(other.publicKey, elsewhere))).toEqual([
        "BAD_REQUEST",
        "PROOF_INVALID",
        "key proof is for another subject or time",
    ]);

    // link the second session with a proof by the registered key
    const link = async (proof: string) =>
        again.client(accountPackage).client.link({
            userId: owner.id,
            id: laptop.id,
            requestId: RequestId.create(fixture.store.environment),
            proof,
        });
    expect(
        await refusal(
            link(await prove(other, laptop.id, fixture.store.clock.now(), fixture.environment)),
        ),
    ).toEqual([
        "FORBIDDEN",
        "KEY_PROOF_REFUSED",
        "key proof does not match an active key of the client",
    ]);
    const proof = await prove(pair, laptop.id, fixture.store.clock.now(), fixture.environment);
    await link(proof);
    expect(await linked()).toEqual(sortIds([owner.clientId, laptop.id]));

    // refuse the same proof a second time
    expect(await refusal(link(proof))).toEqual([
        "BAD_REQUEST",
        "PROOF_INVALID",
        "key proof was already used",
    ]);

    // hide the client from another user
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    expect(
        await refusal(
            stranger.client(accountPackage).client.get({ userId: owner.id, id: laptop.id }),
        ),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", `client ${laptop.id} not found`]);

    // store the browser's push subscription
    const push = {
        url: "https://push.example.com/subscription",
        keys: { p256dh: RFC_8291_KEY, auth: RFC_8291_AUTH },
    };
    const stored = async () =>
        (
            await fixture.storage.database
                .select({ push: client.table.push })
                .from(client.table)
                .where(eq(client.table.id, browser.id))
        ).map((row) => row.push);
    const subscribed = await owner.client(accountPackage).client.subscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(fixture.store.environment),
        push,
    });
    const refusedPush = await refusal(
        owner.client(accountPackage).client.subscribe({
            userId: owner.id,
            id: laptop.id,
            requestId: RequestId.create(fixture.store.environment),
            push,
        }),
    );
    expect(await stored()).toEqual([push]);
    await owner.client(accountPackage).client.unsubscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(fixture.store.environment),
    });
    expect([subscribed, refusedPush, await stored()]).toEqual([
        { ...browser, revision: browser.revision + 1 },
        ["BAD_REQUEST", "PUSH_BROWSER_REQUIRED", "only browsers can subscribe to push"],
        [null],
    ]);

    // revoke the computer, its key and the session linked to it
    const linkedLaptop = await owner.client(accountPackage).client.get({
        userId: owner.id,
        id: laptop.id,
    });
    const revoked = await owner.client(accountPackage).client.revoke({
        userId: owner.id,
        id: laptop.id,
        requestId: RequestId.create(fixture.store.environment),
    });
    const ended = await fixture.storage.database
        .select({ revokedAt: key.table.revokedAt })
        .from(key.table)
        .where(eq(key.table.id, registered.id));
    expect({
        revoked,
        ended,
        exchanged: await refusal(
            fixture.issuer.exchange(await again.token(), accountPackage.id, {}),
        ),
    }).toEqual({
        revoked: {
            ...linkedLaptop,
            revision: linkedLaptop.revision + 1,
            updatedAt: now,
            revokedAt: now,
        },
        ended: [{ revokedAt: now }],
        exchanged: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            "the token endpoint refused the request: invalid_grant: the person's session ended",
        ],
    });
});

test("refresh a person's access token once it expires, and keep calling as the person", async () => {
    await using fixture = await TestAccountStore.open();
    const person = await fixture.signIn("person@example.com");

    // read the person's account, then again past the access token's expiry
    const read = () =>
        person.client(accountPackage).account.get({ scope: person.id, id: person.account });
    const issued = await person.tokens();
    const before = await read();
    await fixture.simulation.jump(issued.expiresAt + 1);
    const after = await read();
    const refreshed = await person.tokens();

    // call with a new access token and refresh token, reading the same account
    expect({
        account: after,
        isAccessTokenNew: refreshed.accessToken !== issued.accessToken,
        isRefreshTokenNew: refreshed.refreshToken !== issued.refreshToken,
        isCurrent: refreshed.expiresAt > fixture.clock.now(),
    }).toEqual({
        account: before,
        isAccessTokenNew: true,
        isRefreshTokenNew: true,
        isCurrent: true,
    });
});
