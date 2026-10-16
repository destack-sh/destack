import { principal, universe, Caller } from "@destack/access";
import { ColumnCipher, eq, type DatabaseConnection, type Select } from "@destack/db";
import type { IdentityRegistry } from "./registry.ts";
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
} from "@destack/identity";
import {
    ASSERTION_TYPE,
    SPACE_KEY,
    TOKEN_LIFETIME_MILLISECONDS,
    TokenCache,
    TokenIssuer,
} from "@destack/runtime/authentication";
import { type Fetch, SpaceMount } from "@destack/runtime";
import { PackageId } from "@destack/package";
import { present } from "@destack/schema";
import { TOKEN_PATH } from "../authentication/path.ts";
import { TokenResponse } from "../authentication/token.ts";
import { JWT_BEARER } from "../oauth/bearer.ts";
import { Scope, type Subject } from "@destack/sync";
import type { Rewrapper } from "@destack/resource";
import { CallError } from "@destack/runtime/error";
import type { Digest } from "@destack/schema";
import { SignJWT, type JWTPayload } from "jose";
import type { Clock } from "@destack/environment";

/** How long a verifier caches a key set before reading it again: 10 minutes, a JWKS cache's age. */
export const KEY_SET_MILLISECONDS = 10 * 60_000;

/** The algorithm of identity keys: ECDSA over P-256. */
const ALGORITHM = { name: "ECDSA", namedCurve: "P-256" } as const;

/** The label binding an encrypted identity secret to its use and subject. */
const LABEL = "destack identity key v1";

/** The bytes of an identity's root secret: 256 bits, the HKDF-SHA256 output length. */
const ROOT_BYTES = 32;

/** The milliseconds in a second, as JWTs count times in seconds. */
const SECOND_MILLISECONDS = 1000;

/** How long an assertion a space signs lives: a minute, one token request and its retries. */
const ASSERTION_SECONDS = 60;

/** The credential kind of a token the universe's own key signs for the universe. */
const UNIVERSE_KEY = "universe-key";

/** The uses of an identity's secrets: its private keys and its root. */
type KeyUse = "signing" | "rotation" | "root";

/** One row of the identity keys a process stores. */
type IdentityKey = Select<typeof identityKey>;

/** How a space's tokens are issued. */
interface SpaceIssuer {
    /** The issuer signing with the identity's key. */
    readonly issuer: TokenIssuer;
    /** The credential the tokens carry. */
    readonly credential: { readonly kind: string; readonly id: string };
}

/** The keys of the identities a process stores: its spaces' or the universe's. */
export class IdentityKeystore {
    /** The process's keyring encrypting the identities' secrets. */
    readonly keyring: Keyring;
    /** The universe's registry publishing each identity. */
    readonly registry: IdentityRegistry;
    /** The sealing of the keys to the machine a space moves to. */
    readonly rewrap: Rewrapper<typeof identityKey, IdentityKey>;
    /** The clock keys are published and tokens signed by. */
    readonly clock: Clock;
    /** The decrypted signing keys, non-extractable, by subject and thumbprint. */
    readonly #signers = new Map<string, Promise<CryptoKey>>();
    /** The derivations of the identities' decrypted root secrets, non-extractable, by subject. */
    readonly #roots = new Map<string, Promise<Deriver>>();

    /** Store identities' keys under a keyring, publishing the identities in a registry. */
    constructor(keyring: Keyring, registry: IdentityRegistry, clock: Clock) {
        // store the keyring, registry and clock, and seal keys with them
        this.keyring = keyring;
        this.registry = registry;
        this.clock = clock;
        this.rewrap = {
            table: identityKey,
            seal: (row, recipient) =>
                rewrite(row, (use, ciphertext) =>
                    this.#decrypt(row.scope, use, ciphertext, (bytes) =>
                        recipient.seal(bytes, context(row.scope, use)),
                    ),
                ),
            open: (row, recipient) =>
                rewrite(row, async (use, sealed) =>
                    this.#encrypt(
                        row.scope,
                        use,
                        await recipient.open(sealed, context(row.scope, use)),
                    ),
                ),
        };
    }

    /** Open the universe's keystore over its database, starting the universe's identity once. */
    static async universe(
        keyring: Keyring,
        database: DatabaseConnection,
        registry: IdentityRegistry,
        clock: Clock,
    ): Promise<IdentityKeystore> {
        // publish identities in the universe's registry
        const keystore = new IdentityKeystore(keyring, registry, clock);

        // generate the universe's identity on the first start
        if (!(await keystore.hasKeys(database, Scope.universe.id))) {
            await keystore.generate(database, Scope.universe.id);
        }

        return keystore;
    }

    /** Generate an identity's signing keys, rotation keys and root secret. */
    async generate(database: DatabaseConnection, subject: string): Promise<void> {
        // generate and store the keys and the root secret
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
        await this.registry.start(operation);
    }

    /** Forget an identity's keys. */
    async forget(database: DatabaseConnection, subject: string): Promise<void> {
        // delete the keys and forget what was decrypted of them
        await database.delete(identityKey).where(eq(identityKey.scope, subject));
        this.#roots.delete(subject);
        this.#forgetSigners(subject);
    }

    /** Wrap a fetch to send each request with a signed token. */
    fetch(database: DatabaseConnection, subject: Subject, audience: PackageId, send: Fetch): Fetch {
        // sign with the key of the subject's space
        const signing = this.#issuer(database, subject);

        return async (request) => {
            // sign a short token of only the subject
            const now = this.clock.now();
            const { accessToken } = await signing.issuer.issue(
                {
                    caller: Caller.of(subject),
                    credential: {
                        ...signing.credential,
                        audience,
                        verifiedAt: now,
                        expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                    },
                },
                now,
            );

            // send the request with it
            const signed = new Request(request);
            signed.headers.set("authorization", `Bearer ${accessToken}`);

            return send(signed);
        };
    }

    /** Ask an issuer for a subject's token of a package with its assertion (RFC 7523). */
    async token(
        database: DatabaseConnection,
        subject: Subject,
        audience: PackageId,
        issuer: string,
        send: Fetch,
    ): Promise<{ readonly accessToken: string; readonly expiresAt: number }> {
        // post an assertion of the subject for the package, an installation's bound to its space
        const endpoint = `${issuer}${TOKEN_PATH}`;
        const body = new URLSearchParams({
            grant_type: JWT_BEARER,
            assertion: await this.assertion(database, subject, endpoint),
            resource: PackageId.resource(audience),
            ...(principal.installation.is(subject) ? { space: subject.scope } : {}),
        });
        const response = await send(new Request(endpoint, { method: "POST", body }));
        if (!response.ok) {
            throw new CallError("UNAUTHORIZED", {
                message: `the issuer refused ${subject.id}'s assertion: ${await response.text()}`,
            });
        }

        return TokenResponse.granted(TokenResponse.parse(await response.json()));
    }

    /** Wrap a fetch to call a universe package with tokens granted for the subject's assertions. */
    asserting(
        database: DatabaseConnection,
        subject: Subject,
        audience: PackageId,
        issuer: string,
        send: Fetch,
    ): Fetch {
        // cache the granted tokens until shortly before they lapse
        const tokens = new TokenCache();

        return async (request) => {
            // send the request with the subject's granted token
            const token = await tokens.token(subject.id, this.clock.now(), () =>
                this.token(database, subject, audience, issuer, send),
            );
            const signed = new Request(request);
            signed.headers.set("authorization", `Bearer ${token}`);

            return send(signed);
        };
    }

    /** Wrap a fetch to call a package as the universe, bound to the space each request goes to. */
    fetchAsUniverse(
        database: DatabaseConnection,
        issuer: string,
        send: Fetch,
    ): (audience: PackageId) => Fetch {
        // sign as the universe under its issuer, with its identity's key
        const self = universe.reference(Scope.universe.id, Scope.universe.id);
        const tokens = new TokenIssuer({
            authority: { kind: "universe" },
            issuer,
            sign: (claims, header) => this.sign(database, Scope.universe.id, claims, header),
        });

        return (audience) => async (request) => {
            // bind the token to the space below the request's mount, refusing a request to no space
            const space = SpaceMount.parse(new URL(request.url).pathname)?.spaceId;
            if (space === undefined) {
                throw new TypeError(
                    `the universe calls packages in spaces alone, not ${request.url}`,
                );
            }

            // sign a short token of the universe for the package in the space
            const now = this.clock.now();
            const { accessToken } = await tokens.issue(
                {
                    caller: Caller.of(self),
                    credential: {
                        kind: UNIVERSE_KEY,
                        id: Scope.universe.id,
                        audience,
                        scope: space,
                        verifiedAt: now,
                        expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                    },
                },
                now,
            );

            // send the request with it
            const signed = new Request(request);
            signed.headers.set("authorization", `Bearer ${accessToken}`);

            return send(signed);
        };
    }

    /** Sign an assertion of a space or one of its installations for a token endpoint (RFC 7523). */
    async assertion(
        database: DatabaseConnection,
        subject: Subject,
        endpoint: string,
    ): Promise<string> {
        // sign a short assertion as the space, for itself or its installation
        const space = IdentityKeystore.#spaceOf(subject);
        const issuedAt = Math.floor(this.clock.now() / SECOND_MILLISECONDS);
        const claims = {
            iss: space,
            sub: subject.id,
            aud: endpoint,
            iat: issuedAt,
            exp: issuedAt + ASSERTION_SECONDS,
            jti: crypto.randomUUID(),
        };

        return this.sign(database, space, claims, { typ: ASSERTION_TYPE });
    }

    /** Issue tokens as a space this machine stores keys of, signed with its active signing key. */
    issuer(database: DatabaseConnection, space: string): TokenIssuer {
        return new TokenIssuer({
            authority: { kind: "space", space },
            issuer: space,
            sign: (claims, header) => this.sign(database, space, claims, header),
        });
    }

    /** Sign a token's claims with the active signing key of an identity this process stores. */
    async sign(
        database: DatabaseConnection,
        subject: string,
        claims: JWTPayload,
        header?: { readonly typ?: string; readonly cty?: string },
    ): Promise<string> {
        // sign with the active key, named by its thumbprint
        const active = this.#active((await this.#require(database, subject)).signingKeys);
        const kid = await PublicKey.thumbprint(active.publicKey);

        return new SignJWT(claims)
            .setProtectedHeader({ typ: "JWT", ...header, alg: "ES256", kid })
            .sign(await this.#signer(subject, kid, active.privateKey));
    }

    /** Report whether this process has an identity's keys. */
    async hasKeys(database: DatabaseConnection, subject: string): Promise<boolean> {
        return (await this.#find(database, subject)) !== undefined;
    }

    /** Derive an identity's secrets and private keys from its root secret under labels. */
    async deriver(database: DatabaseConnection, subject: string): Promise<Deriver | undefined> {
        // refuse an identity whose keys this process no longer stores, as after a move
        const row = await this.#find(database, subject);
        if (row === undefined) {
            this.#roots.delete(subject);

            return undefined;
        }

        // reuse the root decrypted before, else decrypt it once and forget a failed decryption
        const root = this.#roots.get(subject);
        if (root !== undefined) {
            return root;
        }
        const decrypting = this.#decryptRoot(subject, row.root);
        this.#roots.set(subject, decrypting);
        decrypting.catch(() => this.#roots.delete(subject));

        return decrypting;
    }

    /** Key a database's sensitive columns with the column key its scope's identity root derives. */
    async key(database: DatabaseConnection, subject: string): Promise<void> {
        // skip a database opened with a key, as its identity's root derives it alike
        if (database.state.cipher !== undefined) {
            return;
        }

        // derive the column key from the root of the identity this process stores
        const root = present(await this.deriver(database, subject), `the keys of ${subject}`);
        database.key(await ColumnCipher.derive(root));
    }

    /** Rotate an identity's signing keys, and return the published operation's digest. */
    async rotate(
        database: DatabaseConnection,
        subject: string,
        owners?: readonly PublicKey[],
    ): Promise<Digest> {
        // read the identity the operation follows
        const row = await this.#require(database, subject);
        const identity = await this.#identity(subject);

        // publish a next signing key before the stored ones, signed with this process's rotation key
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
        await this.registry.extend(operation);

        // store the published keys
        await database
            .update(identityKey)
            .set({ signingKeys })
            .where(eq(identityKey.scope, subject));

        return IdentityOperation.digest(operation);
    }

    /** Replace every key of an identity this process stores. */
    async rekey(database: DatabaseConnection, subject: string): Promise<Digest> {
        // read the identity the operation follows
        const row = await this.#require(database, subject);
        const identity = await this.#identity(subject);
        const replaced = await PublicKey.thumbprint(row.rotationKey);

        // publish a fresh signing key alone and this process's fresh rotation key, signed with the replaced one
        const signing = await this.#generateSigningKey(subject);
        const rotation = await crypto.subtle.generateKey(ALGORITHM, true, ["sign", "verify"]);
        const rotationKey = await PublicKey.of(rotation.publicKey);
        const rotationKeys = await Promise.all(
            identity.rotationKeys.map(async (key) =>
                (await PublicKey.thumbprint(key)) === replaced ? rotationKey : key,
            ),
        );
        const operation = await IdentityOperation.sign(
            {
                subject,
                previous: identity.digest,
                signingKeys: [signing.publicKey],
                rotationKeys,
            },
            await this.#privateKey(subject, "rotation", row.rotation),
        );
        await this.registry.extend(operation);

        // store the fresh keys and forget what was decrypted of the replaced ones
        const encrypted = await this.#encryptKey(subject, "rotation", rotation.privateKey);
        await database
            .update(identityKey)
            .set({ signingKeys: [signing], rotation: encrypted, rotationKey })
            .where(eq(identityKey.scope, subject));
        this.#forgetSigners(subject);

        return IdentityOperation.digest(operation);
    }

    /** Encrypt every secret of a root key version under the keyring's active version. */
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
            const { signingKeys, rotation, root } = await rewrite(row, async (use, ciphertext) =>
                this.#encrypt(
                    row.scope,
                    use,
                    await this.keyring.decrypt(ciphertext, context(row.scope, use)),
                ),
            );
            await database
                .update(identityKey)
                .set({ signingKeys, rotation, root })
                .where(eq(identityKey.scope, row.scope));
        }

        return stale.length;
    }

    /** Generate a signing key with its private half encrypted under the keyring, published now. */
    async #generateSigningKey(subject: string): Promise<SigningKey> {
        const pair = await crypto.subtle.generateKey(ALGORITHM, true, ["sign", "verify"]);

        return {
            publicKey: await PublicKey.of(pair.publicKey),
            privateKey: await this.#encryptKey(subject, "signing", pair.privateKey),
            publishedAt: this.clock.now(),
        };
    }

    /** Encrypt a private key under the keyring as PKCS #8 bytes, bound to its identity and use. */
    async #encryptKey(subject: string, use: KeyUse, key: CryptoKey): Promise<Ciphertext> {
        return this.#encrypt(
            subject,
            use,
            new Uint8Array(await crypto.subtle.exportKey("pkcs8", key)),
        );
    }

    /** Encrypt a secret's bytes under the keyring, bound to its identity and use. */
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

    /** Forget the signing keys decrypted for an identity. */
    #forgetSigners(subject: string): void {
        for (const name of this.#signers.keys()) {
            if (name.startsWith(`${subject}\0`)) {
                this.#signers.delete(name);
            }
        }
    }

    /** Choose how a space or one of its installations signs its tokens, refusing anyone else. */
    #issuer(database: DatabaseConnection, subject: Subject): SpaceIssuer {
        const space = IdentityKeystore.#spaceOf(subject);

        return {
            issuer: this.issuer(database, space),
            credential: { kind: SPACE_KEY, id: space },
        };
    }

    /** Read the space that signs for a space or one of its installations, refusing anyone else. */
    static #spaceOf(subject: Subject): string {
        // refuse any signer but a space or its installation
        if (!principal.installation.is(subject) && !principal.space.is(subject)) {
            throw new TypeError(`${subject.type} ${subject.id} signs as no space or installation`);
        }

        return principal.installation.is(subject) ? subject.scope : subject.id;
    }

    /** Choose the signing key that signs now. */
    #active(signingKeys: readonly SigningKey[]): SigningKey {
        // take the newest key published past the verifiers' key set age, else the oldest
        const now = this.clock.now();
        const active =
            signingKeys.find((key) => key.publishedAt + KEY_SET_MILLISECONDS <= now) ??
            signingKeys.at(-1);
        if (active === undefined) {
            throw new CallError("INTERNAL_SERVER_ERROR", { message: "no signing key is stored" });
        }

        return active;
    }

    /** Read the keys this process stores for an identity, refusing one it stores none of. */
    async #require(database: DatabaseConnection, subject: string): Promise<IdentityKey> {
        const row = await this.#find(database, subject);
        if (row === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `this process has no keys of ${subject}`,
            });
        }

        return row;
    }

    /** Read the keys this process stores for an identity, absent for one it stores none of. */
    async #find(database: DatabaseConnection, subject: string): Promise<IdentityKey | undefined> {
        const [row] = await database
            .select()
            .from(identityKey)
            .where(eq(identityKey.scope, subject));

        return row;
    }

    /** Read the decrypted signing key of a thumbprint once, as a non-extractable key that signs. */
    #signer(subject: string, kid: string, privateKey: Ciphertext): Promise<CryptoKey> {
        // reuse the key decrypted before
        const name = `${subject}\0${kid}`;
        const signer = this.#signers.get(name);
        if (signer !== undefined) {
            return signer;
        }

        // decrypt it and forget a failed decryption
        const decrypting = this.#privateKey(subject, "signing", privateKey);
        this.#signers.set(name, decrypting);
        decrypting.catch(() => this.#signers.delete(name));

        return decrypting;
    }

    /** Decrypt a private key under the keyring as a non-extractable key that signs. */
    #privateKey(subject: string, use: KeyUse, ciphertext: Ciphertext): Promise<CryptoKey> {
        return this.#decrypt(subject, use, ciphertext, (bytes) =>
            crypto.subtle.importKey("pkcs8", bytes, ALGORITHM, false, ["sign"]),
        );
    }

    /** Decrypt a secret under the keyring, read its bytes and zero them. */
    async #decrypt<Value>(
        subject: string,
        use: KeyUse,
        ciphertext: Ciphertext,
        read: (bytes: Uint8Array<ArrayBuffer>) => Promise<Value>,
    ): Promise<Value> {
        const bytes = await this.keyring.decrypt(ciphertext, context(subject, use));
        try {
            return await read(bytes);
        } finally {
            bytes.fill(0);
        }
    }

    /** Decrypt an identity's root secret as a non-extractable key that derives. */
    #decryptRoot(subject: string, ciphertext: Ciphertext): Promise<Deriver> {
        return this.#decrypt(subject, "root", ciphertext, async (bytes) =>
            Derivation.of(await Derivation.root(bytes)),
        );
    }

    /** Read an identity's current state in the registry, refusing one without an identity. */
    async #identity(subject: string) {
        const identity = await this.registry.find(subject);
        if (identity === undefined) {
            throw new CallError("NOT_FOUND", { message: `${subject} has no identity` });
        }

        return identity;
    }
}

/** Rewrite every secret of a row: its signing keys, its rotation key and its root. */
async function rewrite(
    row: IdentityKey,
    transform: (use: KeyUse, ciphertext: Ciphertext) => Promise<Ciphertext>,
): Promise<IdentityKey> {
    return {
        ...row,
        signingKeys: await Promise.all(
            row.signingKeys.map(async (key) => ({
                ...key,
                privateKey: await transform("signing", key.privateKey),
            })),
        ),
        rotation: await transform("rotation", row.rotation),
        root: await transform("root", row.root),
    };
}

/** Encode the encryption context of an identity secret: its label, subject and use. */
function context(subject: string, use: KeyUse): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${LABEL}\0${subject}\0${use}`);
}
