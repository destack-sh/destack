import { eq } from "@destack/db";
import { Subject } from "@destack/sync";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { session } from "../src/object/authentication.ts";
import { device, deviceKey, DeviceProof, type ProofRequest } from "../src/object/device.ts";
import { user } from "../src/object/user.ts";
import { Credential } from "../src/object/credential.ts";
import { AccountFixture } from "./fixture.ts";

/** One day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Register a device from a session, prove its keys, attach another session, and end all of them by revoking it. */
test("register devices with proven keys and revoke them with their sessions", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const again = await fixture.signIn("owner@example.com");

    // register the device the first session signs in on, attaching that session
    const laptop = await owner.client.device.create({
        userId: owner.id,
        requestId: RequestId.create(),
        name: "laptop",
        category: "laptop",
        operatingSystem: "macos",
    });
    expect(laptop).toEqual({
        id: laptop.id,
        createdAt: laptop.createdAt,
        createdBy: Subject.key(owner.subject),
        updatedBy: Subject.key(owner.subject),
        updatedAt: laptop.createdAt,
        revision: 1,
        tags: {},
        scope: owner.id,
        name: "laptop",
        category: "laptop",
        operatingSystem: "macos",
        operatingSystemVersion: null,
        architecture: null,
        model: null,
        clientVersion: null,
        lastSeenAt: laptop.createdAt,
        revokedAt: null,
    });
    const attached = async () =>
        (await fixture.opened.database.select().from(session.table))
            .map((row) => String(row.deviceId))
            .sort((left, right) => left.localeCompare(right));
    expect(await attached()).toEqual([laptop.id, "null"]);

    // register a key the device proves it holds, once
    const key = await DeviceKeyPair.generate();
    const register = async (publicKey: DeviceKeyPair["publicKey"], proof: string) =>
        owner.client.deviceKey.create({
            userId: owner.id,
            requestId: RequestId.create(),
            parentId: laptop.id,
            publicKey,
            proof,
        });
    const registered = await register(key.publicKey, await key.prove(laptop.id));
    expect(registered).toEqual({
        id: registered.id,
        createdAt: registered.createdAt,
        createdBy: Subject.key(owner.subject),
        updatedBy: Subject.key(owner.subject),
        updatedAt: registered.createdAt,
        revision: 1,
        tags: {},
        scope: owner.id,
        parentId: laptop.id,
        publicKey: key.publicKey,
        thumbprint: await key.thumbprint(),
        verifiedAt: registered.createdAt,
        expiresAt: registered.createdAt + 365 * DAY,
        revokedAt: null,
    });
    await expect(register(key.publicKey, await key.prove(laptop.id))).rejects.toMatchObject({
        code: "CONFLICT",
        message: "a record with the same unique key exists",
    });

    // refuse proofs for another device or time, by another key, forged, or bound to a request
    const other = await DeviceKeyPair.generate();
    const refusals = await Promise.all(
        [
            other.prove("device-elsewhere"),
            other.prove(laptop.id, Date.now() - 120_000),
            key.prove(laptop.id),
            other.prove(laptop.id).then((proof) => `${proof.slice(0, -4)}AAAA`),
            other.prove(laptop.id, Date.now(), { method: "GET", url: "https://account.test/user" }),
            Promise.resolve("not-a-proof"),
        ].map(async (proof) =>
            register(other.publicKey, await proof).then(
                () => "registered",
                (error: { code: string; message: string }) => [error.code, error.message],
            ),
        ),
    );
    expect(refusals).toEqual([
        ["BAD_REQUEST", "device proof is for another device or time"],
        ["BAD_REQUEST", "device proof is for another device or time"],
        ["BAD_REQUEST", "device proof signature is invalid"],
        ["BAD_REQUEST", "device proof signature is invalid"],
        ["BAD_REQUEST", "device proof is for another request"],
        ["BAD_REQUEST", "device proof is no compact JWS"],
    ]);

    // attach the second session with a proof by the registered key, never by an unregistered one
    const attach = async (proof: string) =>
        again.client.device.attach({
            userId: owner.id,
            id: laptop.id,
            requestId: RequestId.create(),
            proof,
        });
    await expect(attach(await other.prove(laptop.id))).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "device proof refers to no active key of the device",
    });
    const proof = await key.prove(laptop.id);
    await attach(proof);
    expect(await attached()).toEqual([laptop.id, laptop.id]);

    // refuse the same proof a second time
    await expect(attach(proof)).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: "device proof was used before",
    });

    // refuse registering a device from a token without a session
    await fixture.elevate(owner);
    const credential = await Credential.create("personal-access-token");
    await owner.client.personalAccessToken.issue({
        userId: owner.id,
        requestId: RequestId.create(),
        name: "script",
        digest: credential.digest,
        restrictions: [
            { ...device.permission("create"), scope: owner.id },
            { ...user.permission("read"), scope: owner.id },
        ],
        expiresAt: Date.now() + DAY,
    });
    await expect(
        fixture
            .connect(() => ({ authorization: `Bearer ${credential.secret}` }))
            .device.create({ userId: owner.id, requestId: RequestId.create(), name: "script" }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "a device attaches a signed-in session",
    });

    // hide the device from another user
    const stranger = await fixture.signIn("stranger@example.com");
    await expect(
        stranger.client.device.get({ userId: owner.id, id: laptop.id }),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no scope ${owner.id}` });

    // revoke the device, ending its key and both sessions
    const revoked = await owner.client.device.revoke({
        userId: owner.id,
        id: laptop.id,
        requestId: RequestId.create(),
    });
    expect(revoked.revokedAt).toBe(revoked.updatedAt);
    const [ended] = await fixture.opened.database
        .select({ revokedAt: deviceKey.table.revokedAt })
        .from(deviceKey.table)
        .where(eq(deviceKey.table.id, registered.id));
    expect(ended?.revokedAt).toBe(revoked.revokedAt);
    for (const person of [owner, again]) {
        await expect(person.client.authentication.current()).rejects.toMatchObject({
            code: "UNAUTHORIZED",
            message: "the person is not signed in",
        });
    }
});

/** A device's P-256 key pair, signing proofs as the device would. */
class DeviceKeyPair {
    /** The key pair. */
    readonly keys: CryptoKeyPair;
    /** The public key as a JSON Web Key's required members. */
    readonly publicKey: { kty: "EC"; crv: "P-256"; x: string; y: string };

    /** Hold a generated key pair and its public key. */
    private constructor(keys: CryptoKeyPair, publicKey: DeviceKeyPair["publicKey"]) {
        this.keys = keys;
        this.publicKey = publicKey;
    }

    /** Generate a key pair. */
    static async generate(): Promise<DeviceKeyPair> {
        const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
            "sign",
            "verify",
        ]);
        const { x, y } = await crypto.subtle.exportKey("jwk", keys.publicKey);

        return new DeviceKeyPair(keys, { kty: "EC", crv: "P-256", x: x!, y: y! });
    }

    /** Sign a proof for a device at a time, binding a request when it authenticates one. */
    prove(deviceId: string, now = Date.now(), request?: ProofRequest): Promise<string> {
        return DeviceProof.sign(this.keys.privateKey, this.publicKey, deviceId, now, request);
    }

    /** Compute the key's RFC 7638 thumbprint. */
    async thumbprint(): Promise<string> {
        const { crv, kty, x, y } = this.publicKey;
        const digest = await crypto.subtle.digest(
            "SHA-256",
            new TextEncoder().encode(JSON.stringify({ crv, kty, x, y })),
        );

        return new Uint8Array(digest).toBase64({ alphabet: "base64url", omitPadding: true });
    }
}
