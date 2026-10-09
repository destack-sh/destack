import { PrivateJwk } from "../oauth/assertion.ts";

/** The label of the one signature a request sends (RFC 9421 4.1). */
const LABEL = "sig1";

/** HTTP message signatures over a request's method, target URI and content digest (RFC 9421, RFC 9530). */
export const MessageSignature = {
    /** Sign a request with an HMAC secret in base64url or an Ed25519 private key as a JWK's JSON, answering the signed copy. */
    async sign(
        request: Request,
        signer: {
            readonly algorithm: "hmac-sha256" | "ed25519";
            readonly keyId: string;
            readonly key: string;
        },
        now: number,
    ): Promise<Request> {
        // cover the method and the target, and a body's digest (RFC 9530 2)
        const headers = new Headers(request.headers);
        const body = request.body === null ? undefined : await request.arrayBuffer();
        const covered = new Map([
            ["@method", request.method],
            ["@target-uri", request.url],
        ]);
        if (body !== undefined) {
            const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", body)).toBase64();
            const value = `sha-256=:${digest}:`;
            headers.set("content-digest", value);
            covered.set("content-digest", value);
        }

        // write the signature base over the covered components (RFC 9421 2.5)
        const created = Math.floor(now / 1000);
        const names = [...covered.keys()].map((name) => `"${name}"`).join(" ");
        const parameters = `(${names});created=${created};keyid="${signer.keyId}";alg="${signer.algorithm}"`;
        const base = [
            ...[...covered].map(([name, value]) => `"${name}": ${value}`),
            `"@signature-params": ${parameters}`,
        ].join("\n");

        // sign the base and attach the signature with its parameters
        const signature = await signBase(base, signer);
        headers.set("signature-input", `${LABEL}=${parameters}`);
        headers.set("signature", `${LABEL}=:${signature}:`);

        return new Request(request.url, {
            method: request.method,
            headers,
            ...(body === undefined ? {} : { body }),
        });
    },
};

/** Sign a signature base with the signer's algorithm, answering the signature in base64. */
async function signBase(
    base: string,
    signer: { readonly algorithm: "hmac-sha256" | "ed25519"; readonly key: string },
): Promise<string> {
    // sign with an HMAC secret
    const bytes = new TextEncoder().encode(base);
    if (signer.algorithm === "hmac-sha256") {
        const secret = Uint8Array.fromBase64(signer.key, { alphabet: "base64url" });
        const key = await crypto.subtle.importKey(
            "raw",
            secret,
            { name: "HMAC", hash: "SHA-256" },
            false,
            ["sign"],
        );

        return new Uint8Array(await crypto.subtle.sign("HMAC", key, bytes)).toBase64();
    }

    // sign with an Ed25519 private key
    const jwk = PrivateJwk.parse(JSON.parse(signer.key));
    const key = await crypto.subtle.importKey("jwk", jwk, { name: "Ed25519" }, false, ["sign"]);

    return new Uint8Array(await crypto.subtle.sign("Ed25519", key, bytes)).toBase64();
}
