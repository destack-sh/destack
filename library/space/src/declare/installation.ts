import { ResourceName } from "@destack/resource";
import { defineSchema, identifier, schema } from "@destack/schema";
import { Package, PackageName } from "@destack/package";
import { ComputeDefinition } from "@destack/package";
import { SpaceResourceBinding, SpaceSecretBinding } from "./resource.ts";
import { InstallationPolicies } from "../policy/index.ts";

/** Select an installation by configuration key or persistent identifier in the destination space. */
export const SpaceInstallationReference = defineSchema(
    schema.union([
        ResourceName,
        schema.object({
            /** The existing installation checked when applying the configuration. */
            id: identifier("installation"),
        }),
    ]),
);

/** A package installation declared in a configuration. */
export const SpaceInstallation = defineSchema(
    schema.object({
        /** Additional restrictions for this installation and its workloads. */
        policies: InstallationPolicies.optional(),
        /** The package release selected by this configuration. */
        package: Package,
        /** Whether this installation should serve requests. */
        state: schema.enum(["enabled", "suspended"]),
        /** Space-local URL alias, independent of the stable installation key. */
        alias: ResourceName,
        /** Resource selections keyed by declaring package and declaration name. */
        resources: schema.record(PackageName, schema.record(ResourceName, SpaceResourceBinding)),
        /** Secret selections keyed by declaring package and declaration name. */
        secrets: schema.record(PackageName, schema.record(ResourceName, SpaceSecretBinding)),
        /** Workload compute settings checked against package and host policy. */
        compute: schema.record(ResourceName, ComputeDefinition),
        /** User-defined labels. */
        tags: schema.record(schema.string().min(1), schema.string()),
    }),
);
/** A package installation declared in a configuration. */
export type SpaceInstallation = schema.Infer<typeof SpaceInstallation>;
