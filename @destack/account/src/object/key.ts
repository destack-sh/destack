import type { Crypto } from "@destack/environment";
import { Scope } from "@destack/sync";
import { type Credential, none, permission, through, union } from "@destack/access";
import { Condition, type Select, type Snapshot } from "@destack/db";
import { MACHINE_KEY } from "@destack/access/token";
import { account } from "./account.ts";
import { user } from "./user.ts";
import { defineObject, field, type MethodContext, type ObjectType, proof } from "@destack/object";
import { Digest, schema } from "@destack/schema";
import { CompactProof, IdentityError, type Proof, PublicKey } from "@destack/identity";
import { Revocation } from "../credential/revocation.ts";
import { CallError } from "@destack/object";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** How long a key authenticates, a year in milliseconds. */
const LIFETIME_MILLISECONDS = 365 * 24 * 60 * 60 * 1000;

/** A client's public key with a proof of its private half, which the client's machine registers. */
export const KeyRegistration = schema.object({
    /** The client. */
    client: schema.identifier("client"),
    /** The key's public half. */
    publicKey: PublicKey,
    /** The client's proof that it has the private half. */
    proof: CompactProof,
});
/** A client's public key with a proof of its private half, which the client's machine registers. */
export type KeyRegistration = schema.Infer<typeof KeyRegistration>;

/** A revocable authentication key of a client or machine. */
export const key = defineObject(
    {
        name: "key",
        plural: "keys",
        scope: [user, account, "universe"],
        nested: { in: "any", receive: "rotate" },
        fields: {
            /** The public key. */
            publicKey: field.json(PublicKey),
            /** The RFC 7638 JWK thumbprint that identifies the key in proofs and tokens. */
            thumbprint: field.string(schema.string().min(1)),
            /** The time the holder proved possession of the private key. */
            verifiedAt: field.time(),
            /** The time the key stops authenticating. */
            expiresAt: field.time(),
            /** The time the key was withdrawn. */
            revokedAt: field.time().optional(),
            /** The time the key was suspended because its machine was disabled. */
            suspendedAt: field.time().optional(),
        },
        permissions: {
            read: through("parent", "verify"),
            list: union(permission("read"), through("parent", "list")),
            rotate: through("parent", "rotate"),
            revoke: through("parent", "revoke"),
            enroll: none(),
            suspend: none(),
        },
        reserved: ["rotate"],
        constraints: {
            thumbprint: { kind: "unique", on: ["thumbprint"] },
            expiry: { kind: "check", condition: { expiresAt: { gt: { field: "createdAt" } } } },
            verification: {
                kind: "check",
                condition: {
                    verifiedAt: { gte: { field: "createdAt" }, lt: { field: "expiresAt" } },
                },
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("rotate", {
            fields: ["publicKey"],
            input: schema.object({ proof: CompactProof }),
            isPredicted: false,
        }),
        enroll: method.create("enroll", { isInternal: true }),
        suspend: method.mutation({ permission: "suspend", isInternal: true }),
        resume: method.mutation({ permission: "suspend", isInternal: true }),
        revoke: method.mutation({ permission: "revoke" }),
    }),
).implement({
    methods: {
        suspend: (call) => call.update({ suspendedAt: call.now }),
        resume: (call) => call.update({ suspendedAt: null }),
        revoke: (call) => {
            // withdraw the target now, refusing one withdrawn before
            Revocation.requireUnrevoked(call.target, call.object.name);

            return call.update({ revokedAt: call.now });
        },
    },
});
/** A persisted key record. */
export type Key = Select<typeof key.table>;

/** The key columns and reads that proofs, assertions and tokens authenticate with. */
export const Key = {
    /** Describe the columns of a key proven now. */
    async columns(publicKey: PublicKey, now: number, crypto: Crypto) {
        return {
            publicKey,
            thumbprint: await PublicKey.thumbprint(publicKey, crypto),
            verifiedAt: now,
            expiresAt: now + LIFETIME_MILLISECONDS,
        };
    },

    /** Read the unrevoked keys of a client or machine. */
    unrevoked(context: Pick<MethodContext, "read">, parent: ObjectType, parentId: string) {
        return context.read(key).findMany({
            where: {
                parentPackageId: parent.typeReference.packageId,
                parentType: parent.name,
                parentId,
                revokedAt: { isNull: true },
            },
        });
    },

    /** Match the keys standing at a time: unrevoked and unexpired. */
    standing(now: number): Condition {
        return { revokedAt: { isNull: true }, expiresAt: { gt: now } };
    },

    /** Match the keys that authenticate at a time: standing and unsuspended. */
    authenticates(now: number): Condition {
        return { AND: [Key.standing(now), { suspendedAt: { isNull: true } }] };
    },

    /** Refuse a machine's token whose key no longer authenticates at a time. */
    async requireStanding(
        reader: Pick<Snapshot, "read">,
        credential: Credential,
        now: number,
    ): Promise<void> {
        // pass other credentials, and assertions, whose verifier read the standing key
        if (credential.kind !== MACHINE_KEY || credential.tokenId === undefined) {
            return;
        }

        // require the thumbprint's key to authenticate now: unrevoked, unexpired and unsuspended
        const standing = await reader.read(key.table).findMany({
            where: { AND: [{ thumbprint: credential.id }, Key.authenticates(now)] },
        });
        if (standing.length === 0) {
            throw AccountError.of("KEY_REVOKED", "the machine token's key is no longer valid");
        }
    },

    /** Spend a verified proof once in the call's mutation, refusing a proof its key used before. */
    async spend(
        context: Pick<MethodContext, "mutate" | "self" | "environment">,
        presented: Proof,
    ): Promise<void> {
        const spent = JSON.stringify([presented.thumbprint, presented.id]);
        const digest = await Digest.of(spent, context.environment.crypto);
        await context
            .mutate(context.self, [
                proof.call("create", {
                    ...proof.in(Scope.universe.id),
                    id: digest,
                    expiresAt: presented.expiresAt,
                }),
            ])
            .catch((error: unknown) => {
                throw CallError.of(error)?.code === "CONFLICT"
                    ? IdentityError.of("PROOF_INVALID", "key proof was already used")
                    : error;
            });
    },
};
