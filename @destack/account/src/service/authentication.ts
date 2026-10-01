import { identifier, Instant, schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { AuthenticationClaims } from "@destack/service/authentication";
import { PackageId } from "@destack/package";
import { defineProcedure } from "@destack/service/procedure";
import { account } from "../object/account.ts";

/** Display information for an authenticated identity. */
export const CallerProfile = schema.object({
    /** The identity described by this profile, qualified by its scope. */
    subject: Subject,
    /** Current display name, excluding private account fields. */
    name: schema.string(),
    /** The handle a user goes by. */
    handle: schema.string().nullable(),
});
/** Display information for an authenticated identity. */
export type CallerProfile = schema.Infer<typeof CallerProfile>;

/** Current identity, scoped token exchange and privileged credential introspection. */
export const authentication = {
    /** Describe the caller's own authentication and profile. */
    current: defineProcedure({ authentication: "identity", audit: false, permission: null })
        .route({ method: "GET", path: "/authentication" })
        .output(AuthenticationClaims.extend({ profile: CallerProfile })),
    /** Exchange an authenticated session or API credential for a scoped access token. */
    exchange: defineProcedure({ authentication: "identity", audit: "activity", permission: null })
        .route({ method: "POST", path: "/authentication/exchange" })
        .input(
            schema.object({
                /** Exact receiving service package. */
                audience: PackageId,
                /** Space whose current memberships are included. */
                spaceId: identifier("space"),
                /** The user to act as. */
                subject: Subject.optional(),
            }),
        )
        .output(
            schema.object({
                /** Signed, short-lived access token. */
                accessToken: schema.string(),
                /** HTTP authorization scheme. */
                tokenType: schema.literal("Bearer"),
                /** Exclusive expiry in epoch milliseconds. */
                expiresAt: Instant,
            }),
        ),
    /** Verify a presented bearer credential using current global records. */
    verify: defineProcedure({
        authentication: "identity",
        audit: "access",
        permission: account.permission("verify"),
    })
        .route({ method: "POST", path: "/authentication/verify" })
        .input(
            schema.object({
                /** Account authorizing the receiving service. */
                accountId: identifier("account"),
                /** Fixed receiving-service audience. */
                audience: PackageId,
                /** Space whose global memberships should be resolved. */
                spaceId: identifier("space"),
                /** Presented credential, excluded from logs and audit details. */
                token: schema.sensitive(schema.string().min(1).max(8192)),
            }),
        )
        .output(AuthenticationClaims),
};
