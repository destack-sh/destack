import { schema } from "@destack/schema";
import { PackageManifest } from "@destack/package/manifest";
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
    /** Manifest describing the complete build. */
    manifest: PackageManifest,
    /** Authorized HTTP endpoint for downloading the complete package archive. */
    download: schema.url(),
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
        permission: { resource: "destack.build", action: "start" },
        audit: true,
    })
        .route({ method: "POST", path: "/builds" })
        .input(BuildRequest)
        .output(BuildOperation.operation),
};
