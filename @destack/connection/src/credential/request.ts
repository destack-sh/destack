import { AwsSigner, EMPTY_HASH } from "@destack/identity/aws";
import type { Credential, Material, Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import type { ConnectionProtocol } from "../object/connection.ts";
import { MessageSignature } from "../signature/signature.ts";
import type { ClientOf } from "@destack/runtime/client";
import { schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import type connectionPackage from "../package.ts";

/** A TLS client certificate a machine presents on a lent request (RFC 8705). */
export interface ClientCertificate {
    /** The certificate chain in PEM. */
    readonly certificate: string;
    /** The private key in PEM. */
    readonly privateKey: string;
}

/** A request with a connection's credential applied, and the machine's client certificate. */
export interface ConnectionRequest {
    /** The authenticated request. */
    readonly request: Request;
    /** The client certificate, for an mTLS connection. */
    readonly certificate?: ClientCertificate;
}

/** Requests authenticated with a connection's credential, as a machine's egress sends them. */
export const ConnectionRequest = {
    /** Apply a connection's current credential to an outbound request. */
    async of(
        protocol: ConnectionProtocol,
        credential: Credential,
        request: Request,
        now: number,
    ): Promise<ConnectionRequest> {
        const headers = new Headers(request.headers);

        // send a key in its header
        if (protocol.kind === "key") {
            const { key } = material(credential, "key");
            headers.set(
                protocol.header,
                protocol.scheme === undefined ? key : `${protocol.scheme} ${key}`,
            );

            return { request: new Request(request, { headers }) };
        }
        // sign the message
        else if (protocol.kind === "signature") {
            const { key } = material(credential, "signature");
            const signer = { algorithm: protocol.algorithm, keyId: protocol.keyId, key };

            return { request: await MessageSignature.sign(request, signer, now) };
        }
        // sign with AWS Signature Version 4
        else if (protocol.kind === "sigv4") {
            return { request: await sigv4(protocol, material(credential, "sigv4"), request, now) };
        }
        // present the client certificate
        else if (protocol.kind === "mtls") {
            const { certificate, privateKey } = material(credential, "mtls");

            return { request, certificate: { certificate, privateKey } };
        }
        // send the current access token
        else {
            const token = credential.token;
            if (token === undefined) {
                throw new ConnectionError("UNAUTHORIZED", "the connection lacks an access token");
            }

            return ConnectionRequest.token(token, request);
        }
    },

    /** Borrow a connection's credential at its home as the consumer, applied to its request. */
    async borrow(
        home: ClientOf<typeof connectionPackage>,
        reference: ObjectReference,
        request: Request,
        now: number,
    ): Promise<ConnectionRequest> {
        const { protocol, credential } = await home.connection.lend({
            scope: reference.scope,
            id: schema.identifier("connection").parse(reference.id),
        });

        return ConnectionRequest.of(protocol, credential, request, now);
    },

    /** Send an access token in the Authorization header under its type's scheme (RFC 6750 2.1). */
    token(token: Token, request: Request): ConnectionRequest {
        const headers = new Headers(request.headers);
        headers.set("authorization", `${scheme(token.tokenType)} ${token.accessToken}`);

        return { request: new Request(request, { headers }) };
    },
};

/** Read the material of a kind a credential has, refusing another. */
function material<const Kind extends Material["kind"]>(
    credential: Credential,
    kind: Kind,
): Extract<Material, { kind: Kind }> {
    const held = credential.material;
    if (held === undefined || !isKind(held, kind)) {
        throw new ConnectionError("UNAUTHORIZED", `the connection lacks ${kind} material`);
    }

    return held;
}

/** Report whether material is of a kind. */
function isKind<const Kind extends Material["kind"]>(
    held: Material,
    kind: Kind,
): held is Extract<Material, { kind: Kind }> {
    return held.kind === kind;
}

/** Sign a request with AWS Signature Version 4, with the body hashed and the path encoded. */
async function sigv4(
    protocol: Extract<ConnectionProtocol, { kind: "sigv4" }>,
    held: Extract<Material, { kind: "sigv4" }>,
    request: Request,
    now: number,
): Promise<Request> {
    // hash the body and sign the request's headers with its Host header
    const url = new URL(request.url);
    const body = request.body === null ? undefined : await request.arrayBuffer();
    const payloadHash =
        body === undefined ? EMPTY_HASH : await AwsSigner.payloadHash(new Uint8Array(body));
    const isS3 = protocol.service === "s3";
    const headers: Record<string, string> = {
        ...Object.fromEntries(request.headers),
        host: url.host,
        ...(isS3 ? { "x-amz-content-sha256": payloadHash } : {}),
    };

    // sign them with the path encoded once for S3
    const signer = new AwsSigner({
        region: protocol.region,
        service: protocol.service,
        credentials: held,
    });
    const signed = await signer.authorize(
        {
            method: request.method,
            path: AwsSigner.path(url.pathname, isS3 ? "single" : "double"),
            query: [...url.searchParams],
            headers,
            payloadHash,
        },
        new Date(now),
    );

    return new Request(request.url, {
        method: request.method,
        headers: signed,
        ...(body === undefined ? {} : { body }),
    });
}

/** Write a token type as the Authorization scheme, Bearer in its registered case (RFC 6750 1). */
function scheme(tokenType: string): string {
    return tokenType.toLowerCase() === "bearer" ? "Bearer" : tokenType;
}
