import { PackageInspection } from "@destack/package/code";
import { schema } from "@destack/schema";
import { defineProcedure, defineService } from "@destack/service";
import { checkout, preview } from "../object/index.ts";
import type {} from "@destack/package/import-meta";

/** The development workflow on a machine: its registered checkouts, the previews running their packages, and inspections with the machine's warm compilers. */
export const buildService = defineService("build", {
    objects: { checkout, preview },
    /** Inspect a package directory of a registered checkout for one of its outputs, with the machine's warm compiler. */
    inspect: defineProcedure({
        authentication: "identity",
        permission: checkout.permission("read"),
        audit: "access",
    })
        .route({ method: "POST", path: "/inspect" })
        .input(
            schema.object({
                /** The checkout with the package. */
                checkout: schema.identifier("checkout"),
                /** The package directory, relative to the checkout's root. */
                directory: schema.string().min(1),
                /** The output whose runtime the inspection compiles for. */
                output: schema.string().min(1),
            }),
        )
        .output(PackageInspection),
});
