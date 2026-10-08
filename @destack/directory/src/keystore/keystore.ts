import { principal, universe } from "@destack/access";
import { eq, type DatabaseConnection } from "@destack/db";
import { type Directory, KEY_SET_MILLISECONDS } from "../directory/directory.ts";
import type { Placement } from "../placement/placement.ts";
import { identityKey, type SigningKey } from "./key.ts";
import {
    type Ciphertext,
    Derivation,
    type Deriver,
    IdentityOperation,
    type Keyring,
    LocalKeyring,
    MAX_SIGNING_KEYS,
    PublicKey,
    type Recipient,
} from "@destack/identity";
import {
    SPACE_KEY,
    AUTHENTICATION_LIFETIME_MILLISECONDS,
    Authentication,
    TokenIssuer,
} from "@destack/service/authentication";
import type { Fetch } from "@destack/service";
import type { PackageId } from "@destack/package";
import { Scope, type Subject } from "@destack/sync";
import type { Rewrapper } from "@destack/resource";
import { ServiceError } from "@destack/service/error";
import type { Digest } from "@destack/schema";
import { SignJWT, type JWTPayload } from "jose";

/** The algorithm of identity keys: ECDSA over P-256. */
const ALGORITHM = { name: "ECDSA", namedCurve: "P-256" } as const;

/** The label binding an encrypted identity secret to its use and subject. */
const LABEL = "destack identity key v1";

/** The bytes of an identity's root secret: 256 bits, the HKDF-SHA256 output length. */
const ROOT_BYTES = 32;

/** The credential kind of a token the universe's own key signs for the universe. */
const UNIVERSE_KEY = "universe-key";

/** The uses of an identity's secrets: its private keys and its root. */
type KeyUse = "signing" | "rotation" | "root";

/** The keys a process keeps for one identity. */
type HeldKeys = typeof identityKey.$inferSelect;

/** The issuer the universe's tokens name, and the one space each of its tokens is bound to (RFC 8707). */
export interface UniverseBinding {
    /** The universe's issuer, such as its sign-in's base URL. */
    readonly issuer: string;
    /** The space the tokens are for. */
    readonly space: string;
}

/** How a subject's tokens are issued: by whose key, under which credential, bound to which space. */
interface Issuance {
    /** The issuer signing with the identity's key. */
    readonly issuer: TokenIssuer;
    /** The credential the tokens name. */
    readonly credential: { readonly kind: string; readonly id: string };
    /** The space the tokens are bound to, absent for the tokens a space signs. */
    readonly scope?: string;
}

/** The keys of the identities a process holds, its spaces' or the universe's: generated with each identity, decrypted only to sign or derive, and sealed to the machine a space moves to. */
export class IdentityKeystore {
    /** The process's keyring encrypting the identities' secrets. */
    readonly keyring: Keyring;
    /** The universe's directory publishing each identity. */
    readonly directory: Directory;
    /** The sealing of the keys to the machine a space moves to. */
    readonly rewrap: Rewrapper<typeof identityKey, HeldKeys>;
    /** The decrypted signing keys, non-extractable, by subject and thumbprint. */
    readonly #signers = new Map<string, Promise<CryptoKey>>();
    /** The derivations of the identities' decrypted root secrets, non-extractable, by subject. */
    readonly #roots = new Map<string, Promise<Deriver>>();
    /** The keystore's clock, in UTC epoch milliseconds. */
    readonly #clock: () => number;

    /** Hold identities' keys under a keyring, publishing the identities in a directory. */
    constructor(keyring: Keyring, directory: Directory, clock: () => number = () => Date.now()) {
        // keep the keyring, directory and clock, and seal keys with them
        this.keyring = keyring;
        this.directory = directory;
        this.#clock = clock;
        this.rewrap = {
            table: identityKey,
            seal: (row, recipient) => this.#seal(row, recipient),
            open: (row, recipient) => this.#open(row, recipient),
        };
    }

    /** Generate an identity's signing and rotation keys and root secret, and start it in the directory: a space's from the machine serving it, the universe's from the process hosting the directory. */
    async generate(
        database: DatabaseConnection,
        subject: string,
        placement?: Placement,
    ): Promise<void> {
        // generate and keep the keys and the root secret
        const signing = await this.#generateSigningKey(subject);
        const rotation = await crypto.subtle.generateKey(ALGORITHM, true, ["sign", "verify"]);
        const rotationKey = await PublicKey.of(rotation.publicKey);
        const row = {
            scope: subject,
            signingKeys: [signing],
            rotation: await this.#encryptKey(subject, "rotation", rotation.privateKey),
            rotationKey,
            root: await this.#encrypt(
                subject,
                "root",
                crypto.getRandomValues(new Uint8Array(ROOT_BYTES)),
            ),
        };
        await database
            .insert(identityKey)
            .values(row)
            .onConflictDoUpdate({ target: identityKey.scope, set: row });

        // start the identity signed by the process's rotation key
        const operation = await IdentityOperation.sign(
            {
                subject,
                previous: null,
                signingKeys: [signing.publicKey],
                rotationKeys: [rotationKey],
            },
            rotation.privateKey,
        );
        await this.directory.apply(operation, placement);
    }

    /** Forget an identity's keys. */
    async forget(database: DatabaseConnection, subject: string): Promise<void> {
        // delete the keys and forget what was decrypted of them
        await database.delete(identityKey).where(eq(identityKey.scope, subject));
        this.#roots.delete(subject);
        for (const name of this.#signers.keys()) {
            if (name.startsWith(`${subject}\0`)) {
                this.#signers.delete(name);
            }
        }
    }

    /** Wrap a fetch to send each request as a space or one of its installations, with a token the space's key signs for the audience, or as the universe to one space, with a token the universe's key signs under its issuer. */
    fetch(
        database: DatabaseConnection,
        subject: Subject,
        audience: PackageId,
        send: Fetch,
        binding?: UniverseBinding,
    ): Fetch {
        // sign with the key of the subject's identity
        const issuance = this.#issuance(database, subject, binding);

        return async (request) => {
            // sign a short token of only the subject
            const now = Date.now();
            const { accessToken } = await issuance.issuer.issue(
                new Authentication({
                    credential: issuance.credential,
                    audience,
                    ...(issuance.scope === undefined ? {} : { scope: issuance.scope }),
                    subject,
                    subjects: [subject],
                    verifiedAt: now,
                    expiresAt: now + AUTHENTICATION_LIFETIME_MILLISECONDS,
                }),
                now,
            );

            // send the request with it
            const signed = new Request(request);
            signed.headers.set("authorization", `Bearer ${accessToken}`);

            return send(signed);
        };
    }

    /** Choose how a subject's tokens are issued: the universe signs as itself for the one space it binds to, a space as itself for itself and its installations. */
    #issuance(
        database: DatabaseConnection,
        subject: Subject,
        binding: UniverseBinding | undefined,
    ): Issuance {
        // sign as the universe under its issuer for one space
        if (universe.is(subject) && binding !== undefined) {
            const issuer = new TokenIssuer({
                authority: { kind: "universe" },
                issuer: binding.issuer,
                sign: (claims) => this.sign(database, Scope.universe.id, claims),
            });

            return {
                issuer,
                credential: { kind: UNIVERSE_KEY, id: Scope.universe.id },
                scope: binding.space,
            };
        }
        // sign as a space for itself or one of its installations
        else if (
            binding === undefined &&
            (principal.installation.is(subject) || principal.space.is(subject))
        ) {
            const space = principal.installation.is(subject) ? subject.scope : subject.id;

            return {
                issuer: this.issuer(database, space),
                credential: { kind: SPACE_KEY, id: space },
            };
        }
        // refuse any other signer, and a universe without its issuer and space
        else {
            throw new TypeError(
                `${subject.type} ${subject.id} signs as no space, installation or bound universe`,
            );
        }
    }

    /** Issue tokens as a space this machine keeps keys of, signed with its active signing key. */
    issuer(database: DatabaseConnection, space: string): TokenIssuer {
        return new TokenIssuer({
            authority: { kind: "space", space },
            issuer: space,
            sign: (claims) => this.sign(database, space, claims),
        });
    }

    /** Sign a token's claims as an identity this process keeps keys of, with its active signing key named by its thumbprint. */
    async sign(
        database: DatabaseConnection,
        subject: string,
        claims: JWTPayload,
        header?: { readonly typ?: string; readonly cty?: string },
    ): Promise<string> {
        const active = this.#active((await this.#require(database, subject)).signingKeys);
        const kid = await PublicKey.thumbprint(active.publicKey);

        return new SignJWT(claims)
            .setProtectedHeader({ typ: "JWT", ...header, alg: "ES256", kid })
            .sign(await this.#signer(subject, kid, active.privateKey));
    }

    /** Report whether this process keeps an identity's keys. */
    async holds(database: DatabaseConnection, subject: string): Promise<boolean> {
        return (await this.#find(database, subject)) !== undefined;
    }

    /** Derive the secrets and private keys of an identity under labels from its root secret, such as its bucket credentials and push keys, absent for one this process keeps no keys of. */
    async deriver(database: DatabaseConnection, subject: string): Promise<Deriver | undefined> {
        // refuse an identity whose keys this process no longer keeps, as after a move
        const row = await this.#find(database, subject);
        if (row === undefined) {
            this.#roots.delete(subject);

            return undefined;
        }

        // reuse the root decrypted before, else decrypt it once and forget a failed decryption
        const kept = this.#roots.get(subject);
        if (kept !== undefined) {
            return kept;
        }
        const decrypting = this.#decryptRoot(subject, row.root);
        this.#roots.set(subject, decrypting);
        decrypting.catch(() => this.#roots.delete(subject));

        return decrypting;
    }

    /** Decrypt an identity's root secret as a root key that derives and never leaves. */
    async #decryptRoot(subject: string, ciphertext: Ciphertext): Promise<Deriver> {
        const bytes = await this.keyring.decrypt(ciphertext, context(subject, "root"));
        try {
            return Derivation.of(await Derivation.root(bytes));
        } finally {
            bytes.fill(0);
        }
    }

    /** Rotate an identity's signing keys: publish a next key, which signs once every verifier read it, keeping the active and the previous ones published, optionally with owners' rotation keys ahead of this process's, and answer the published operation's digest. */
    async rotate(
        database: DatabaseConnection,
        subject: string,
        owners?: readonly PublicKey[],
    ): Promise<Digest> {
        // read the identity the operation follows
        const row = await this.#require(database, subject);
        const identity = await this.#identity(subject);

        // publish a next signing key before the kept ones, signed with this process's rotation key
        const generated = await this.#generateSigningKey(subject);
        const signingKeys = [generated, ...row.signingKeys].slice(0, MAX_SIGNING_KEYS);
        const operation = await IdentityOperation.sign(
            {
                subject,
                previous: identity.digest,
                signingKeys: signingKeys.map((key) => key.publicKey),
                rotationKeys:
                    owners === undefined ? identity.rotationKeys : [...owners, row.rotationKey],
            },
            await this.#privateKey(subject, "rotation", row.rotation),
        );
        await this.directory.apply(operation);

        // keep the published keys
        await database
            .update(identityKey)
            .set({ signingKeys })
            .where(eq(identityKey.scope, subject));

        return IdentityOperation.digest(operation);
    }

    /** Encrypt every secret kept under a root key version under the keyring's active one instead, answering how many identities changed. */
    async reencrypt(database: DatabaseConnection, version: number): Promise<number> {
        // read the identities with a secret under the version
        const rows = await database.select().from(identityKey);
        const isStale = (ciphertext: Ciphertext) => LocalKeyring.version(ciphertext) === version;
        const stale = rows.filter(
            (row) =>
                isStale(row.rotation) ||
                isStale(row.root) ||
                row.signingKeys.some((key) => isStale(key.privateKey)),
        );

        // encrypt each of their secrets under the active version
        for (const row of stale) {
            const reencrypt = async (use: KeyUse, ciphertext: Ciphertext) =>
                this.#encrypt(
                    row.scope,
                    use,
                    await this.keyring.decrypt(ciphertext, context(row.scope, use)),
                );
            await database
                .update(identityKey)
                .set({
                    signingKeys: await Promise.all(
                        row.signingKeys.map(async (key) => ({
                            ...key,
                            privateKey: await reencrypt("signing", key.privateKey),
                        })),
                    ),
                    rotation: await reencrypt("rotation", row.rotation),
                    root: await reencrypt("root", row.root),
                })
                .where(eq(identityKey.scope, row.scope));
        }

        return stale.length;
    }

    /** Read the keys this process keeps for an identity, absent for one it keeps none of. */
    async #find(database: DatabaseConnection, subject: string): Promise<HeldKeys | undefined> {
        const [row] = await database
            .select()
            .from(identityKey)
            .where(eq(identityKey.scope, subject));

        return row;
    }

    /** Read the keys this process keeps for an identity, refusing one it keeps none of. */
    async #require(database: DatabaseConnection, subject: string): Promise<HeldKeys> {
        const row = await this.#find(database, subject);
        if (row === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `this process holds no keys of ${subject}`,
            });
        }

        return row;
    }

    /** Read an identity's current state in the directory, refusing one without an identity. */
    async #identity(subject: string) {
        const identity = await this.directory.identity(subject);
        if (identity === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${subject} has no identity` });
        }

        return identity;
    }

    /** Generate a signing key with its private half encrypted under the keyring, published now. */
    async #generateSigningKey(subject: string): Promise<SigningKey> {
        const pair = await crypto.subtle.generateKey(ALGORITHM, true, ["sign", "verify"]);

        return {
            publicKey: await PublicKey.of(pair.publicKey),
            privateKey: await this.#encryptKey(subject, "signing", pair.privateKey),
            publishedAt: this.#clock(),
        };
    }

    /** Choose the signing key that signs now: the newest every verifier read, else the oldest, as an identity's first key. */
    #active(signingKeys: readonly SigningKey[]): SigningKey {
        // take the newest key published past the verifiers' key set age
        const now = this.#clock();
        const active =
            signingKeys.find((key) => key.publishedAt + KEY_SET_MILLISECONDS <= now) ??
            signingKeys.at(-1);
        if (active === undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", { message: "no signing key is kept" });
        }

        return active;
    }

    /** Read the decrypted signing key of a thumbprint once, as a key that signs and never leaves. */
    #signer(subject: string, kid: string, privateKey: Ciphertext): Promise<CryptoKey> {
        // reuse the key decrypted before
        const name = `${subject}\0${kid}`;
        const kept = this.#signers.get(name);
        if (kept !== undefined) {
            return kept;
        }

        // decrypt it and forget a failed decryption
        const decrypting = this.#privateKey(subject, "signing", privateKey);
        this.#signers.set(name, decrypting);
        decrypting.catch(() => this.#signers.delete(name));

        return decrypting;
    }

    /** Encrypt a private key under the keyring as PKCS #8 bytes, bound to its identity and use. */
    async #encryptKey(subject: string, use: KeyUse, key: CryptoKey): Promise<Ciphertext> {
        return this.#encrypt(
            subject,
            use,
            new Uint8Array(await crypto.subtle.exportKey("pkcs8", key)),
        );
    }

    /** Encrypt a secret's bytes under the keyring, bound to its identity and use, and erase them. */
    async #encrypt(
        subject: string,
        use: KeyUse,
        bytes: Uint8Array<ArrayBuffer>,
    ): Promise<Ciphertext> {
        try {
            return await this.keyring.encrypt(bytes, context(subject, use));
        } finally {
            bytes.fill(0);
        }
    }

    /** Decrypt a private key under the keyring as a key that signs and never leaves. */
    async #privateKey(subject: string, use: KeyUse, ciphertext: Ciphertext): Promise<CryptoKey> {
        const bytes = await this.keyring.decrypt(ciphertext, context(subject, use));
        try {
            return await crypto.subtle.importKey("pkcs8", bytes, ALGORITHM, false, ["sign"]);
        } finally {
            bytes.fill(0);
        }
    }

    /** Seal a row's secrets to the recipient of the machine a space moves to. */
    async #seal(row: HeldKeys, recipient: Recipient): Promise<HeldKeys> {
        const seal = async (use: KeyUse, ciphertext: Ciphertext): Promise<Ciphertext> => {
            const bytes = await this.keyring.decrypt(ciphertext, context(row.scope, use));
            try {
                return await recipient.seal(bytes, context(row.scope, use));
            } finally {
                bytes.fill(0);
            }
        };

        return {
            ...row,
            signingKeys: await Promise.all(
                row.signingKeys.map(async (key) => ({
                    ...key,
                    privateKey: await seal("signing", key.privateKey),
                })),
            ),
            rotation: await seal("rotation", row.rotation),
            root: await seal("root", row.root),
        };
    }

    /** Open a row's secrets sealed to this machine's recipient and encrypt them under this process's keyring. */
    async #open(row: HeldKeys, recipient: Recipient): Promise<HeldKeys> {
        const open = async (use: KeyUse, sealed: Ciphertext): Promise<Ciphertext> =>
            this.#encrypt(row.scope, use, await recipient.open(sealed, context(row.scope, use)));

        return {
            ...row,
            signingKeys: await Promise.all(
                row.signingKeys.map(async (key) => ({
                    ...key,
                    privateKey: await open("signing", key.privateKey),
                })),
            ),
            rotation: await open("rotation", row.rotation),
            root: await open("root", row.root),
        };
    }
}

/** Encode the encryption context of an identity secret: its label, subject and use. */
function context(subject: string, use: KeyUse): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${LABEL}\0${subject}\0${use}`);
}
