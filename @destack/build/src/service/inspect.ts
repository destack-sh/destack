import { schema } from "@destack/schema";
import { PackageInspection } from "@destack/package/inspect";
import { defineProcedure } from "@destack/service";

/** Select source and a named compiler configuration for inspection. */
export const InspectRequest = schema.object({
    /** Source reference resolved and authorized by the host. */
    source: schema.string().min(1),
    /** Named output configuration determining target and runtime. */
    output: schema.string().min(1),
});

/** Source inspection selection. */
export type InspectRequest = schema.Infer<typeof InspectRequest>;

/** Inspect source through the same isolated toolchain used for builds. */
export const inspect = defineProcedure({
    authentication: "identity",
    permission: { resource: "destack.build", action: "inspect" },
    audit: true,
})
    .route({ method: "POST", path: "/inspect" })
    .input(InspectRequest)
    .output(PackageInspection);
