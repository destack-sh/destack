import { PackageId } from "@destack/package";
import accountPackage from "../package.ts";
import { OAuthError } from "../authentication/token.ts";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";

/** How long each package's access tokens live, a minute unless the universe sets the package's. */
export class TokenLifetime {
    /** The lifetimes the universe sets, in milliseconds, by package. */
    readonly #byPackage: ReadonlyMap<PackageId, number>;

    /** Store the lifetimes the universe sets. */
    constructor(byPackage: ReadonlyMap<PackageId, number>) {
        this.#byPackage = byPackage;
    }

    /** Read how long a package's tokens live, in milliseconds. */
    of(audience: PackageId): number {
        return this.#byPackage.get(audience) ?? TOKEN_LIFETIME_MILLISECONDS;
    }
}

/** The packages as the resources the token endpoint issues tokens for (RFC 8707). */
export const PackageResource = {
    /** Read the package of a resource identifier, absent for any other identifier. */
    package(identifier: string): PackageId | undefined {
        // take the identifier's last segment as the package when its resource is the identifier
        const id = PackageId.safeParse(identifier.slice(identifier.lastIndexOf(":") + 1));

        return id.success && PackageId.resource(id.data) === identifier ? id.data : undefined;
    },

    /** Read the package a resource asks tokens for, the account package without one. */
    audience(identifier: string | undefined): PackageId {
        // ask for the account package without a resource
        if (identifier === undefined) {
            return accountPackage.id;
        }

        // refuse a resource of no package
        const audience = PackageResource.package(identifier);
        if (audience === undefined) {
            throw OAuthError.of("INVALID_TARGET", `${identifier} names no package`);
        }

        return audience;
    },
};
