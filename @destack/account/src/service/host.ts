import { PackageId } from "@destack/package";
import { identifier, Instant, schema } from "@destack/schema";
import { defineProcedure } from "@destack/service/procedure";

/** The access tokens hosts are granted by proving their keys. */
export const hostToken = {
    /** Grant a host an access token for a service, in a space it serves or in the universe. */
    grant: defineProcedure({ authentication: "public", permission: null, audit: false })
        .route({ method: "POST", path: "/hosts/token" })
        .input(
            schema.object({
                /** The host's assertion, a proof by its key bound to this request. */
                assertion: schema.string().min(1),
                /** The receiving service's package. */
                audience: PackageId,
                /** The space the host acts in, absent for universe services. */
                spaceId: identifier("space").optional(),
            }),
        )
        .output(
            schema.object({
                /** The signed, short-lived access token. */
                accessToken: schema.string(),
                /** The HTTP authorization scheme. */
                tokenType: schema.literal("Bearer"),
                /** The exclusive expiry in epoch milliseconds. */
                expiresAt: Instant,
            }),
        ),
};
