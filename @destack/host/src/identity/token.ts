import { principal } from "@destack/access";
import type { Directory } from "@destack/directory";
import type { PackageId } from "@destack/package";
import { identifier, type Identifier, type schema } from "@destack/schema";
import { type Caller, TokenAuthentication, TokenVerifier } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import type { HostKey } from "../object/host.ts";

/** The installation tokens the holders of spaces sign with their host keys. */
export class SpaceToken {
    /** Report whether a request carries a space token: a bearer token whose issuer is a host. */
    static accepts(request: Request): boolean {
        const payload = SpaceToken.#payload(request);
        if (payload === undefined) {
            return false;
        }

        // read the issuer, a host for space tokens and the universe's URL for user tokens
        try {
            const claims = JSON.parse(
                new TextDecoder().decode(Uint8Array.fromBase64(payload, { alphabet: "base64url" })),
            ) as { iss?: unknown };

            return identifier("host").safeParse(claims.iss).success;
        } catch {
            return false;
        }
    }

    /** Verify an installation token its space's holder signed, for a package's service in a space. */
    static async verify(
        request: Request,
        options: {
            /** The universe's directory of zones and cells. */
            readonly directory: Directory;
            /** Read the keys of an account's host authenticating at a time. */
            readonly keys: (
                accountId: Identifier<"account">,
                hostId: Identifier<"host">,
                now: number,
            ) => Promise<readonly Pick<HostKey, "publicKey" | "thumbprint">[]>;
            /** The package receiving the call. */
            readonly audience: PackageId;
            /** The space the call targets, the installation's own for universe services. */
            readonly spaceId?: string;
        },
        now = Date.now(),
    ): Promise<Caller> {
        // read the claimed signer and installation before verifying them
        const claims = SpaceToken.#claims(request);
        const signer = identifier("host").parse(claims.iss);
        const installation = claims.caller.subject;

        // require an installation whose space the signer holds
        if (!principal.installation.is(installation)) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "the token's caller is no installation",
            });
        }
        const zone = await options.directory.locate(installation.scope);
        const cell = zone?.cell === signer ? await options.directory.cell(signer) : undefined;
        if (cell === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "the token's signer holds no space of its installation",
            });
        }

        // verify the token with the signer's active keys as the space's authority
        const keys = await options.keys(identifier("account").parse(cell.scope), signer, now);
        const verifier = new TokenVerifier({
            authority: { kind: "space", spaceId: installation.scope },
            issuer: signer,
            audience: options.audience,
            keys: {
                keys: keys.map((key) => ({
                    ...key.publicKey,
                    kid: key.thumbprint,
                    alg: "ES256",
                })),
            },
        });

        return verifier.authenticate(request, options.spaceId ?? installation.scope, now);
    }

    /** Read a bearer token's issuer and caller without verifying them, refusing any other credential. */
    static #claims(request: Request): {
        iss: string;
        caller: schema.Infer<typeof TokenAuthentication>;
    } {
        // require one bearer token of three parts
        const payload = SpaceToken.#payload(request);
        if (payload === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        // decode its claims
        try {
            const claims = JSON.parse(
                new TextDecoder().decode(Uint8Array.fromBase64(payload, { alphabet: "base64url" })),
            ) as { iss: string; caller: unknown };

            return { iss: claims.iss, caller: TokenAuthentication.parse(claims.caller) };
        } catch (error) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "invalid access token",
                cause: error,
            });
        }
    }

    /** Read the encoded claims of a request's bearer token, absent for any other credential. */
    static #payload(request: Request): string | undefined {
        const [, token] = /^Bearer (\S+)$/.exec(request.headers.get("authorization") ?? "") ?? [];
        const [, payload] = token?.split(".") ?? [];

        return payload;
    }
}
