import { PackageId } from "@destack/package";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";

/** The milliseconds in a second, as OAuth counts expiries in seconds. */
const SECOND_MILLISECONDS = 1000;

/** The model Better Auth reads protected resources from, which packages answer. */
export const RESOURCE_MODEL = "oauthResource";

/** The model Better Auth reads client-resource links from, which no row fills. */
export const CLIENT_RESOURCE_MODEL = "oauthClientResource";

/** A package as the OAuth resource its access tokens are for (RFC 8707). */
export interface PackageResource {
    /** The resource's identifier, `urn:destack:package:<packageId>`. */
    readonly id: string;
    /** The identifier its tokens include as `aud`. */
    readonly identifier: string;
    /** The name, the package's identifier. */
    readonly name: string;
    /** The access-token lifetime in seconds. */
    readonly accessTokenTtl: number;
    /** The refresh-token lifetime in seconds, the provider's default. */
    readonly refreshTokenTtl: null;
    /** The signing algorithm, the provider's. */
    readonly signingAlgorithm: null;
    /** The signing key, the active one. */
    readonly signingKeyId: null;
    /** The scopes, any the client asks. */
    readonly allowedScopes: null;
    /** The claims a resource adds, none. */
    readonly customClaims: null;
    /** Whether tokens must be bound to a proof-of-possession key, never. */
    readonly dpopBoundAccessTokensRequired: false;
    /** Whether issuance is disabled, never. */
    readonly disabled: false;
}

/** The packages as the resources Better Auth issues tokens for. */
export const PackageResource = {
    /** Read the package of a resource identifier, absent for any other identifier. */
    package(identifier: string): PackageId | undefined {
        // take the identifier's last segment as the package when its resource is the identifier
        const id = PackageId.safeParse(identifier.slice(identifier.lastIndexOf(":") + 1));

        return id.success && PackageId.resource(id.data) === identifier ? id.data : undefined;
    },

    /** Answer a resource identifier as its package's resource, absent for any other identifier. */
    of(identifier: string, lifetimes: ReadonlyMap<PackageId, number>): PackageResource | undefined {
        // refuse anything but a package
        const id = PackageResource.package(identifier);
        if (id === undefined) {
            return undefined;
        }

        // live as long as the package's verifiers accept
        const lifetime = lifetimes.get(id) ?? TOKEN_LIFETIME_MILLISECONDS;

        return {
            id: identifier,
            identifier,
            name: id,
            accessTokenTtl: lifetime / SECOND_MILLISECONDS,
            refreshTokenTtl: null,
            signingAlgorithm: null,
            signingKeyId: null,
            allowedScopes: null,
            customClaims: null,
            dpopBoundAccessTokensRequired: false,
            disabled: false,
        };
    },
};
