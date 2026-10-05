import type { PackageId } from "@destack/package";
import { copyRequest, RequestId } from "@destack/service/request";
import { type Identifier, schema } from "@destack/schema";
import { DeviceProof, DevicePublicKey, type ProofRequest } from "@destack/account/object";
import { connect } from "@destack/account/client";
import type { Host } from "@destack/account/object";
import { Keychain } from "../keychain/index.ts";

/** A private P-256 key as Web Crypto exports it, a JSON Web Key keeping its private scalar beside the coordinates. */
const PrivateJwk = schema.looseObject({
    /** The key type. */
    kty: schema.literal("EC"),
    /** The curve. */
    crv: schema.literal("P-256"),
    /** The x coordinate, base64url encoded. */
    x: schema.string(),
    /** The y coordinate, base64url encoded. */
    y: schema.string(),
});
/** A private P-256 key as a JSON Web Key. */
type PrivateJwk = schema.Infer<typeof PrivateJwk>;

/** How long before its expiry a host re-grants an access token, in milliseconds: the verifiers' clock tolerance and a grant's round trip. */
const TOKEN_REFRESH_MILLISECONDS = 10_000;

/** The key algorithm hosts sign with. */
const KEY_ALGORITHM = { name: "ECDSA", namedCurve: "P-256" } as const;

/** What a host enrolls with beside its key. */
export type Enrollment = Omit<
    Parameters<ReturnType<typeof connect>["host"]["enroll"]>[0],
    "id" | "publicKey" | "proof"
>;

/** The space or placement a host's token binds, the universe without either (RFC 8707). */
export type TokenBinding = Pick<
    Parameters<ReturnType<typeof connect>["hostToken"]["grant"]>[0],
    "spaceId" | "placementId"
>;

/** A host's key pair. */
interface KeyPair {
    /** The public half. */
    readonly publicKey: DevicePublicKey;
    /** The private half signing proofs. */
    readonly privateKey: CryptoKey;
}

/** The identity a host proves. */
export class HostIdentity {
    /** The host's identifier. */
    readonly hostId: Identifier<"host">;
    /** The keychain keeping the private key. */
    readonly #keys: Keychain;
    /** The key pair once loaded or generated. */
    #pair: KeyPair | undefined;
    /** The access tokens granted to this host, by audience and binding, pending while a grant runs. */
    readonly #tokens = new Map<string, Promise<{ accessToken: string; expiresAt: number }>>();
    /** The key rotations in progress in this process, by host. */
    static readonly #rotations = new Map<string, Promise<void>>();

    /** Prove a host's identity with the key its store keeps. */
    constructor(hostId: Identifier<"host">, keys: Keychain) {
        this.hostId = hostId;
        this.#keys = keys;
    }

    /** Generate and keep a key pair, and return its public half with a proof of possession. */
    async generate(now = Date.now()): Promise<{ publicKey: DevicePublicKey; proof: string }> {
        // replace the kept pair, and prove possession of the pair kept
        const kept = await this.#keys.load(this.hostId);
        const pair = await this.#replace(kept, await HostIdentity.#generate());

        return { publicKey: pair.publicKey, proof: await this.prove(now) };
    }

    /** Enroll this host with a new key through a client the approving user signed in. */
    async enroll(
        client: ReturnType<typeof connect>,
        enrollment: Enrollment,
        now = Date.now(),
    ): Promise<Host> {
        const { publicKey, proof } = await this.generate(now);

        return client.host.enroll({
            ...enrollment,
            id: this.hostId,
            publicKey,
            proof,
        });
    }

    /** Register a new key through a client signed with the current one and keep it, joining a rotation of this host in progress. */
    async rotate(
        client: ReturnType<typeof connect>,
        accountId: string,
        now = Date.now(),
    ): Promise<void> {
        // run a rotation of this host, forgetting it once it settles
        const pending = HostIdentity.#rotations.get(this.hostId);
        if (pending === undefined) {
            const rotation = this.#rotate(client, accountId, now).finally(() =>
                HostIdentity.#rotations.delete(this.hostId),
            );
            HostIdentity.#rotations.set(this.hostId, rotation);
            await rotation;
        }
        // join the rotation in progress
        else {
            await pending;
        }

        // reload the kept key, and forget the tokens the replaced key granted
        this.#pair = undefined;
        this.#tokens.clear();
    }

    /** Register a new key through a client signed with the current one, then keep it. */
    async #rotate(
        client: ReturnType<typeof connect>,
        accountId: string,
        now: number,
    ): Promise<void> {
        // generate a new pair beside the kept one
        const kept = await this.#keys.load(this.hostId);
        const generated = await HostIdentity.#generate();
        const { pair } = generated;

        // register the new pair with a proof by its private half
        await client.hostKey.create({
            accountId: schema.identifier("account").parse(accountId),
            requestId: RequestId.create(),
            parentId: this.hostId,
            publicKey: pair.publicKey,
            proof: await DeviceProof.sign(pair.privateKey, pair.publicKey, this.hostId, now),
        });

        // keep the new pair unless a racing writer kept another
        await this.#replace(kept, generated);
    }

    /** Sign a proof for this host now, bound to a request or proving possession of the key. */
    async prove(now = Date.now(), request?: ProofRequest): Promise<string> {
        const pair = await this.#load();

        return DeviceProof.sign(pair.privateKey, pair.publicKey, this.hostId, now, request);
    }

    /** Wrap a fetch to call a service with this host's access tokens for a binding. */
    fetch(
        send: (request: Request) => Promise<Response>,
        audience: PackageId,
        accounts: string,
        binding: TokenBinding = {},
    ): (request: Request) => Promise<Response> {
        return async (request) => {
            // replace the request's credential with the token for the audience
            const { accessToken } = await this.token(audience, accounts, send, binding);
            const headers = new Headers(request.headers);
            headers.set("authorization", `Bearer ${accessToken}`);

            return send(copyRequest(request, { headers }));
        };
    }

    /** Read this host's access token for a service and binding, granting a new one shortly before the last expires. */
    async token(
        audience: PackageId,
        accounts: string,
        fetch: (request: Request) => Promise<Response>,
        binding: TokenBinding = {},
        now = Date.now(),
    ): Promise<{ accessToken: string; expiresAt: number }> {
        // reuse a pending or fresh token
        const key = JSON.stringify([
            audience,
            binding.spaceId ?? null,
            binding.placementId ?? null,
        ]);
        const cached = this.#tokens.get(key);
        const current = await cached;
        if (current !== undefined && current.expiresAt - TOKEN_REFRESH_MILLISECONDS > now) {
            return current;
        }

        // grant a new one with an assertion bound to the grant request, dropping it when refused
        const granting = (async () => {
            const assertion = await this.prove(now, {
                method: "POST",
                url: `${accounts}/hosts/token`,
            });

            return connect({ url: accounts, fetch }).hostToken.grant({
                assertion,
                audience,
                ...binding,
            });
        })();
        this.#tokens.set(key, granting);
        granting.catch(() => this.#tokens.delete(key));

        return granting;
    }

    /** Forget the host's private key. */
    async forget(): Promise<void> {
        await this.#keys.remove(this.hostId);
        this.#pair = undefined;
        this.#tokens.clear();
    }

    /** Keep a generated pair in place of the key read before, or adopt the key a racing writer kept. */
    async #replace(
        kept: string | undefined,
        generated: { pair: KeyPair; privateJwk: PrivateJwk },
    ): Promise<KeyPair> {
        // keep the generated pair unless another writer kept a key meanwhile
        const privateJwk = JSON.stringify(generated.privateJwk);
        const current = await Keychain.update(this.#keys, this.hostId, (key) =>
            key === kept || key === undefined ? privateJwk : key,
        );
        if (current === privateJwk) {
            this.#pair = generated.pair;

            return generated.pair;
        }

        // adopt the key the other writer kept
        this.#pair = await HostIdentity.#read(current);

        return this.#pair;
    }

    /** Load the kept key pair and refuse a host without one. */
    async #load(): Promise<KeyPair> {
        // reuse the pair once loaded
        if (this.#pair !== undefined) {
            return this.#pair;
        }

        // read the private half from the store
        const kept = await this.#keys.load(this.hostId);
        if (kept === undefined) {
            throw new TypeError(`host ${this.hostId} has no key`);
        }
        this.#pair = await HostIdentity.#read(kept);

        return this.#pair;
    }

    /** Import the key pair a keychain keeps as a private JSON Web Key. */
    static #read(kept: string): Promise<KeyPair> {
        const stored: unknown = JSON.parse(kept);

        return HostIdentity.#import(PrivateJwk.parse(stored));
    }

    /** Generate a key pair, returning it with its private half as a JSON Web Key. */
    static async #generate(): Promise<{ pair: KeyPair; privateJwk: PrivateJwk }> {
        // generate an extractable pair and export its private half
        const generated = await crypto.subtle.generateKey(KEY_ALGORITHM, true, ["sign", "verify"]);
        const exported = await crypto.subtle.exportKey("jwk", generated.privateKey);
        const privateJwk = PrivateJwk.parse(exported);

        return { pair: await HostIdentity.#import(privateJwk), privateJwk };
    }

    /** Import a private key for signing, with the public half from its coordinates. */
    static async #import(privateJwk: PrivateJwk): Promise<KeyPair> {
        const privateKey = await crypto.subtle.importKey("jwk", privateJwk, KEY_ALGORITHM, false, [
            "sign",
        ]);
        const publicKey = DevicePublicKey.parse({
            kty: privateJwk.kty,
            crv: privateJwk.crv,
            x: privateJwk.x,
            y: privateJwk.y,
        });

        return { publicKey, privateKey };
    }
}
