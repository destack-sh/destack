import { type Authentication, none, principal, through } from "@destack/access";
import { type AssertionKey, Bearer, TokenVerifier } from "@destack/runtime/authentication";
import type { PackageId } from "@destack/package";
import { createLocalJWKSet } from "jose";
import { Scope } from "@destack/sync";
import type { Select, Snapshot } from "@destack/db";
import { defineObject, defineShape, field } from "@destack/object";
import { IdentityOperation, KeySet, PublicKey } from "@destack/identity";
import { Key } from "./key.ts";
import { Digest, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";

/** The longest signed operation, in characters: eight public keys with room to spare. */
const OPERATION_LENGTH = 16_384;

/** A signed identity operation, a compact JSON Web Signature. */
const SignedOperation = schema.string().min(1).max(OPERATION_LENGTH);

/** A space's or the universe's identity, the registry entry its signed operations change. */
export const identity = defineObject(
    {
        name: "identity",
        plural: "identities",
        scope: "universe",
        key: schema.string().min(1),
        fields: {},
        // let the universe's roles grant reading and extending identities, signatures deciding each change
        permissions: { read: none(), list: none(), start: none(), extend: none() },
    },
    (method) => ({
        /** Start an identity with its first signed operation. */
        create: method.create("start", {
            fields: [],
            input: schema.object({ operation: SignedOperation }),
            isPredicted: false,
        }),

        /** Read the keys that verify what the identity signs, as a JSON Web Key Set. */
        keys: method.query({ permission: "read", output: KeySet }),
    }),
);
/** An identity as its table stores it. */
export type IdentityRow = Select<typeof identity.table>;

/** A signed change of an identity's keys, in the identity's log. */
export const operation = defineObject(
    {
        name: "operation",
        identity: "identity-operation",
        plural: "operations",
        scope: "universe",
        nested: { in: identity, delete: "restrict", receive: "extend" },
        fields: {
            /** The position in the identity's log, from zero. */
            sequence: field.integer(),
            /** The digest of the signed operation, which the next operation refers to. */
            digest: field.string(Digest),
            /** The signed operation. */
            operation: field.string(SignedOperation),
            /** The signing rotation key's priority among the keys of the operation it follows. */
            priority: field.integer(),
            /** When the registry applied it, in UTC epoch milliseconds. */
            appliedAt: field.time(),
            /** Whether a higher-priority rotation key nullified it. */
            isNullified: field.boolean().default(false),
        },
        permissions: {
            read: through("parent", "read"),
            list: through("parent", "read"),
            extend: through("parent", "extend"),
        },
        constraints: {
            sequence: { kind: "unique", on: ["parentId", "sequence"] },
            digest: { kind: "index", on: ["digest"] },
        },
    },
    (method) => ({
        /** Append a signed operation to its identity's log, nullifying later ones it recovers. */
        create: method.create("extend", {
            fields: [],
            input: schema.object({ operation: SignedOperation }),
            isPredicted: false,
        }),

        /** Mark an operation nullified by a later one a higher-priority rotation key signed. */
        nullify: method.update("extend", {
            fields: ["isNullified"],
            isInternal: true,
            isPredicted: false,
        }),
    }),
);
/** An identity operation as its table stores it. */
export type Operation = Select<typeof operation.table>;

/** The identities and their logs, replicated to verify what the universe and spaces sign. */
export const identityShape = defineShape({
    name: "identity",
    parameters: schema.object({}),
    objects: [identity, operation],
    queries: () => [identity.query({ scopes: "every" }), operation.query({ scopes: "every" })],
});

/** What a token is verified against: the receiving package, issuer, space and time. */
export interface Verification {
    /** The package receiving the call. */
    readonly audience: PackageId;
    /** The universe's issuer, which its tokens carry. */
    readonly universe: string;
    /** The space the call targets, absent for a call outside any one space. */
    readonly scope?: string;
    /** The time the token is verified at. */
    readonly now: number;
}

/** The reads of identities from their logs. */
export const Identity = {
    /** List an identity's log in order, nullified operations included. */
    async log(reader: Pick<Snapshot, "read">, subject: string): Promise<Operation[]> {
        return reader.read(operation.table).findMany({
            where: { parentId: subject },
            orderBy: { sequence: "asc" },
        });
    },

    /** Read an identity's state from its latest operation in force, absent before its first. */
    current(log: readonly Pick<Operation, "operation" | "digest" | "isNullified">[]) {
        // read the latest operation in force, which carries the identity's whole key state
        const latest = log.findLast((entry) => !entry.isNullified);
        if (latest === undefined) {
            return undefined;
        }
        const { signingKeys, rotationKeys } = IdentityOperation.read(latest.operation);

        return { signingKeys, rotationKeys, digest: latest.digest };
    },

    /** Read a space's signing key by an assertion's thumbprint, absent for none. */
    async assertionKey(
        snapshot: Snapshot,
        thumbprint: string,
        spaceId: string,
    ): Promise<AssertionKey | undefined> {
        // find the key among the space's current signing keys
        const current = Identity.current(await Identity.log(snapshot, spaceId));
        const thumbprints = await Promise.all(
            (current?.signingKeys ?? []).map(async (key) => ({
                key,
                thumbprint: await PublicKey.thumbprint(key),
            })),
        );
        const found = thumbprints.find((entry) => entry.thumbprint === thumbprint);

        return found === undefined
            ? undefined
            : {
                  id: thumbprint,
                  subject: principal.space.reference(Scope.universe.id, spaceId),
                  publicKey: found.key,
              };
    },

    /** Read the key set verifying what an identity signs, refusing one without operations. */
    async keys(reader: Pick<Snapshot, "read">, subject: string): Promise<KeySet> {
        const found = Identity.current(await Identity.log(reader, subject));
        if (found === undefined) {
            throw new CallError("NOT_FOUND", { message: `${subject} has no identity` });
        }

        return PublicKey.keySet(found.signingKeys);
    },

    /** Verify a request's bearer token and a machine token's key, absent without a token. */
    async authenticate(
        snapshot: Snapshot,
        request: Request,
        options: Verification,
    ): Promise<Authentication | undefined> {
        // call anonymously without a bearer token
        const token = Bearer.token(request.headers.get("authorization"));
        if (token === undefined) {
            return undefined;
        }

        // verify the token, and the key of a machine's token
        const authentication = await Identity.verify(snapshot, token, options);
        await Key.requireStanding(snapshot, authentication.credential, options.now);

        return authentication;
    },

    /** Verify a token by its issuer's identity keys: the universe's, or a space's own. */
    async verify(
        snapshot: Snapshot,
        token: string,
        options: Verification,
    ): Promise<Authentication> {
        // read the issuing universe or space, refusing any other issuer
        const issuer = schema.string().safeParse(Bearer.issuer(token));
        const space = schema.identifier("space").safeParse(issuer.data);
        const signer =
            issuer.data === options.universe
                ? { authority: { kind: "universe" } as const, subject: Scope.universe.id }
                : space.success
                  ? {
                        authority: { kind: "space", space: space.data } as const,
                        subject: space.data,
                    }
                  : undefined;
        if (!issuer.success || signer === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "invalid access token" });
        }

        // verify the token by the signer's keys
        const verifier = new TokenVerifier({
            authority: signer.authority,
            issuer: issuer.data,
            audience: options.audience,
            keys: createLocalJWKSet(await Identity.keys(snapshot, signer.subject)),
        });

        return verifier.verify(token, options.now, options.scope);
    },
};
