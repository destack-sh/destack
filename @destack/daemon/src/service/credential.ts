import { PackageId } from "@destack/package";
import { PermissionReference } from "@destack/access";
import { schema } from "@destack/schema";
import { defineProcedure } from "@destack/service";

/** A local client's explicit authorization, approved by the host owner. */
export const ClientAuthorization = schema.object({
    /** Package consuming the granted services. */
    packageId: PackageId,
    /** Exact operations allowed within this host. */
    permissions: schema.array(PermissionReference).max(256),
});
/** A local client's explicit authorization. */
export type ClientAuthorization = schema.Infer<typeof ClientAuthorization>;

/** Host-only creation and revocation of local client credentials. */
export const credential = {
    /** Create a process-local credential with explicit operation restrictions. */
    create: defineProcedure({ authentication: "host", permission: null, audit: true })
        .route({ method: "POST", path: "/credentials" })
        .input(ClientAuthorization)
        .output(schema.object({ token: schema.string() })),
    /** Revoke a credential and reject subsequent calls and stream disclosures. */
    revoke: defineProcedure({ authentication: "host", permission: null, audit: true })
        .route({ method: "DELETE", path: "/credentials" })
        .input(schema.object({ token: schema.string() }))
        .output(schema.object({})),
};
