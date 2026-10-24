import type { Crypto } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { Base64Url } from "./base64.ts";
import { JwsAlgorithm } from "./algorithm.ts";

/** The members of each key type that its RFC 7638 thumbprint hashes, in order. */
const THUMBPRINT_MEMBERS: ReadonlyMap<string, readonly string[]> = new Map([
    ["EC", ["crv", "kty", "x", "y"]],
    ["OKP", ["crv", "kty", "x"]],
    ["RSA", ["e", "kty", "n"]],
    ["oct", ["k", "kty"]],
]);

/** The names of the Ed25519 signature algorithm, polymorphic and fully specified. */
const EDWARDS: readonly string[] = ["EdDSA", "Ed25519"];

/** A JSON Web Key, read for the members that choose and import it (RFC 7517 4). */
const jwkSchema = schema.looseObject({
    /** The key type. */
    kty: schema.string().min(1),
    /** The curve of an elliptic or Edwards key. */
    crv: schema.string().min(1).exactOptional(),
    /** The key's identifier. */
    kid: schema.string().min(1).exactOptional(),
    /** The algorithm it is for. */
    alg: schema.string().min(1).exactOptional(),
    /** Its use, signatures or encryption. */
    use: schema.string().min(1).exactOptional(),
});
/** A JSON Web Key. */
export type Jwk = schema.Infer<typeof jwkSchema>;

/** JSON Web Keys: exported, matched to their algorithms and identified by thumbprint. */
export const Jwk = Object.assign(jwkSchema, {
    /** Export an extractable key as a JSON Web Key. */
    async export(key: CryptoKey, crypto: Crypto): Promise<Jwk> {
        return jwkSchema.parse(await crypto.exportKey("jwk", key));
    },

    /** Compute a key's RFC 7638 thumbprint from its type's required members. */
    async thumbprint(jwk: Jwk, crypto: Crypto): Promise<string> {
        // order the key type's required members by name, refusing a missing one
        const members = THUMBPRINT_MEMBERS.get(jwk.kty);
        if (members === undefined) {
            throw IdentityError.of("KEY_INVALID", `a ${jwk.kty} key has no thumbprint`);
        }
        const missing = members.find((member) => typeof jwk[member] !== "string");
        if (missing !== undefined) {
            throw IdentityError.of("KEY_INVALID", `the ${jwk.kty} key has no ${missing}`);
        }
        const ordered = Object.fromEntries(members.map((member) => [member, jwk[member]]));

        // hash their JSON with SHA-256
        const json = new TextEncoder().encode(JSON.stringify(ordered));
        const digest = await crypto.digest("SHA-256", json);

        return Base64Url.encode(new Uint8Array(digest));
    },

    /** Read the algorithm a key signs with: its own, else its type and curve's first. */
    algorithm(jwk: Jwk): JwsAlgorithm | undefined {
        return jwk.alg === undefined
            ? JwsAlgorithm.of(jwk.kty, jwk.crv)
            : JwsAlgorithm.safeParse(jwk.alg).data;
    },

    /** Report whether a key belongs to an algorithm: its type, curve, algorithm and use. */
    fits(jwk: Jwk, algorithm: JwsAlgorithm): boolean {
        // take EdDSA and Ed25519 as one algorithm of Ed25519 keys (RFC 9864 2)
        const { kty, crv } = JwsAlgorithm.parameters(algorithm);
        const named = crv === "Ed25519" ? EDWARDS : [algorithm];

        return (
            jwk.kty === kty &&
            (crv === undefined || jwk.crv === crv) &&
            (jwk.alg === undefined || named.includes(jwk.alg)) &&
            (jwk.use === undefined || jwk.use === "sig")
        );
    },
});
