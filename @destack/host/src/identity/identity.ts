import type { webcrypto } from "node:crypto";
import { copyRequest, RequestId } from "@destack/service/request";
import { identifier } from "@destack/schema";
import { DeviceProof, DevicePublicKey, type ProofRequest } from "@destack/account/object";
import type { connect } from "../client/index.ts";
import type { Host } from "../object/index.ts";
import type { Keychain } from "../keychain/index.ts";

/** A private key as Web Crypto exports it, a JSON Web Key. */
type PrivateJwk = webcrypto.JsonWebKey;

/** The authorization scheme of a request a host signs with a proof by its key. */
export const HOST_PROOF_SCHEME = "HostProof";

/** The key algorithm hosts sign with. */
const KEY_ALGORITHM = { name: "ECDSA", namedCurve: "P-256" } as const;

/** What a host enrolls with beside its key. */
export type Enrollment = Omit<
    Parameters<ReturnType<typeof connect>["host"]["enroll"]>[0],
    "id" | "publicKey" | "proof"
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
    readonly hostId: string;
    /** The keychain keeping the private key. */
    readonly #keys: Keychain;
    /** The key pair once loaded or generated. */
    #pair: KeyPair | undefined;

    /** Prove a host's identity with the key its store keeps. */
    constructor(hostId: string, keys: Keychain) {
        this.hostId = hostId;
        this.#keys = keys;
    }

    /** Generate and keep a key pair, returning its public half with a proof of possession. */
    async generate(now = Date.now()): Promise<{ publicKey: DevicePublicKey; proof: string }> {
        // keep a new pair, and prove possession of it
        const { pair, privateJwk } = await HostIdentity.#generate();
        await this.#keys.save(this.hostId, JSON.stringify(privateJwk));
        this.#pair = pair;

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
        }) as Promise<Host>;
    }

    /** Register a new key through a client signed with the current one, then keep it. */
    async rotate(
        client: ReturnType<typeof connect>,
        accountId: string,
        now = Date.now(),
    ): Promise<void> {
        // register a new pair with a proof by its own private half, then keep it
        const { pair, privateJwk } = await HostIdentity.#generate();
        await client.hostKey.create({
            accountId: identifier("account").parse(accountId),
            requestId: RequestId.create(),
            parentId: identifier("host").parse(this.hostId),
            publicKey: pair.publicKey,
            proof: await DeviceProof.sign(pair.privateKey, pair.publicKey, this.hostId, now),
        });
        await this.#keys.save(this.hostId, JSON.stringify(privateJwk));
        this.#pair = pair;
    }

    /** Sign a proof for this host now, bound to a request or proving possession of the key. */
    async prove(now = Date.now(), request?: ProofRequest): Promise<string> {
        const pair = await this.#load();

        return DeviceProof.sign(pair.privateKey, pair.publicKey, this.hostId, now, request);
    }

    /** Sign a request as this host, replacing any credential it carries. */
    async sign(request: Request, now = Date.now()): Promise<Request> {
        // replace the request's credential with a proof bound to it
        const headers = new Headers(request.headers);
        const proof = await this.prove(now, { method: request.method, url: request.url });
        headers.set("authorization", `${HOST_PROOF_SCHEME} ${proof}`);

        return copyRequest(request, { headers });
    }

    /** Wrap a fetch to sign each request as this host. */
    fetch(inner: (request: Request) => Promise<Response>): (request: Request) => Promise<Response> {
        return async (request) => inner(await this.sign(request));
    }

    /** Forget the host's private key. */
    async forget(): Promise<void> {
        await this.#keys.remove(this.hostId);
        this.#pair = undefined;
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
            throw new TypeError(`host ${this.hostId} holds no key`);
        }
        this.#pair = await HostIdentity.#import(JSON.parse(kept) as PrivateJwk);

        return this.#pair;
    }

    /** Generate a key pair, returning it with its private half as a JSON Web Key. */
    static async #generate(): Promise<{ pair: KeyPair; privateJwk: PrivateJwk }> {
        const generated = await crypto.subtle.generateKey(KEY_ALGORITHM, true, ["sign", "verify"]);
        const privateJwk = await crypto.subtle.exportKey("jwk", generated.privateKey);

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
