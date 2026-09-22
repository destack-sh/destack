import { PackageId } from "@destack/package/package";
import packageDefinition from "../../destack.json" with { type: "json" };
import { schema } from "@destack/schema";
import { PackageLocation } from "@destack/package/manifest";
import { defineOperation, defineOperationProcedures } from "@destack/service/operation";
import { defineProcedure } from "@destack/service";

/** Select immutable source and named outputs from the host's build configuration. */
export const BuildRequest = schema.object({
    /** Source reference resolved to an immutable checkout by the host. */
    source: schema.string().min(1),
    /** Named output configurations selected from that source. */
    outputs: schema.array(schema.string().min(1)).min(1),
});

/** Build selection supplied by a client. */
export type BuildRequest = schema.Infer<typeof BuildRequest>;

/** Complete package stored by the host after successful compilation. */
export const BuildResult = schema.object({
    /** Source reference selected by the request. */
    source: schema.string().min(1),
    /** Stored package available independently of the compiler's lifetime. */
    package: PackageLocation,
});

/** Stored build returned to clients. */
export type BuildResult = schema.Infer<typeof BuildResult>;

/** Current build phase, without estimating work remaining. */
export const BuildProgress = schema.object({
    phase: schema.enum(["preparing", "building", "storing"]),
});

/** Current build phase. */
export type BuildProgress = schema.Infer<typeof BuildProgress>;

/** Observable build state shared by procedures and the runner. */
export const BuildOperation = defineOperation(BuildResult, BuildProgress);

/** Start and observe finite package builds. */
export const build = {
    ...defineOperationProcedures(BuildOperation, "/builds"),
    start: defineProcedure({
        authentication: "identity",
        permission: {
            packageId: PackageId.parse(packageDefinition.id),
            type: "build",
            name: "start",
        },
        audit: true,
    })
        .route({ method: "POST", path: "/builds" })
        .input(BuildRequest)
        .output(BuildOperation.operation),
};
