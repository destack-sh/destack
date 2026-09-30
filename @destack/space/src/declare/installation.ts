import { ComputeDefinition, DeclarationName, Package, PackageId } from "@destack/package";
import { Digest, PackagePath } from "@destack/package/file";
import { defineSchema, identifier, schema } from "@destack/schema";
import { SpaceBinding } from "./resource.ts";
import { InstallationPolicy } from "../policy/index.ts";
import type { PackageHandle } from "@destack/package/declare";
import { SpaceError } from "../error/index.ts";

/** A package directory within a repository or checkout, or its root. */
const PackageDirectory = schema.union([schema.literal("."), PackagePath]);

/** A committed Git object identifier. */
const Commit = schema.string().regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/);

/** The release, repository reference or checkout an installation follows. */
export const InstallationSelection = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Follow one immutable registry release. */
            kind: schema.literal("release"),
            /** The selected package version. */
            version: schema.string().min(1),
        }),
        schema.object({
            /** Follow committed repository history. */
            kind: schema.literal("repository"),
            /** The registered repository. */
            repository: identifier("repository"),
            /** The full branch or tag reference. */
            reference: schema.string().regex(/^refs\/(heads|tags)\/.+$/),
            /** The package directory within the repository. */
            directory: PackageDirectory,
        }),
        schema.object({
            /** Follow a local working directory, including unpublished changes. */
            kind: schema.literal("checkout"),
            /** The host administering the checkout. */
            host: identifier("host"),
            /** The checkout registered on that host. */
            checkout: identifier("checkout"),
            /** The package directory within the checkout. */
            directory: PackageDirectory,
        }),
    ]),
);
/** The release, repository reference or checkout an installation follows. */
export type InstallationSelection = schema.Infer<typeof InstallationSelection>;

/** The exact build a revision evaluated. */
export const InstallationBuild = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** A published registry release. */
            kind: schema.literal("release"),
            /** The released package version. */
            version: schema.string().min(1),
            /** The digest of the release manifest, absent before a host resolved the release. */
            manifest: Digest.optional(),
        }),
        schema.object({
            /** A build of committed repository history. */
            kind: schema.literal("commit"),
            /** The registered repository retaining the commit. */
            repository: identifier("repository"),
            /** The complete Git commit object identifier. */
            commit: Commit,
            /** The package directory within the repository. */
            directory: PackageDirectory,
            /** The digest of the build manifest. */
            manifest: Digest,
        }),
        schema.object({
            /** A build of a local checkout, including unpublished changes. */
            kind: schema.literal("checkout"),
            /** The host retaining the checkout and build. */
            host: identifier("host"),
            /** The registered working directory. */
            checkout: identifier("checkout"),
            /** The parent commit, absent for an unborn branch. */
            commit: Commit.optional(),
            /** The package directory within the checkout. */
            directory: PackageDirectory,
            /** The digest of the build manifest, absent when evaluated from source. */
            manifest: Digest.optional(),
        }),
    ]),
);
/** The exact build a revision evaluated. */
export type InstallationBuild = schema.Infer<typeof InstallationBuild>;

/** Select an installation by configuration key or persistent identifier in the destination space. */
export const SpaceInstallationReference = defineSchema(
    schema.union([
        DeclarationName,
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
        policies: InstallationPolicy.optional(),
        /** The package release selected by this configuration. */
        package: Package,
        /** Whether this installation should serve requests. */
        status: schema.enum(["enabled", "suspended"]),
        /** Space-local URL alias, independent of the stable installation key. */
        alias: DeclarationName,
        /** The bindings of the package's declarations, keyed by immutable package ID and declaration name. */
        bindings: schema.record(PackageId, schema.record(DeclarationName, SpaceBinding)),
        /** Workload compute settings checked against package and host policy. */
        compute: schema.record(DeclarationName, ComputeDefinition),
        /** User-defined labels. */
        tags: schema.record(schema.string().min(1), schema.string()),
    }),
);
/** A package installation declared in a configuration. */
export type SpaceInstallation = schema.Infer<typeof SpaceInstallation>;

/** A space secret selected for a package secret, optionally pinned to one version. */
export type SecretSelection = string | { readonly secret: string; readonly version: number };

/** Space resource and secret keys selected for a package's declarations. */
export type InstallBindingMap<Handle extends PackageHandle> = {
    readonly [Name in keyof Handle["resources"]]?: string;
} & { readonly [Name in keyof Handle["secrets"]]?: SecretSelection };

/** Installation settings other than bindings. */
export interface InstallOptions {
    /** Space-local URL alias; defaults to the package name without its scope. */
    readonly alias?: string;
    /** Whether the installation serves requests; defaults to enabled. */
    readonly status?: SpaceInstallation["status"];
    /** Additional restrictions for this installation and its workloads. */
    readonly policies?: SpaceInstallation["policies"];
    /** Workload compute settings. */
    readonly compute?: SpaceInstallation["compute"];
    /** User-defined labels. */
    readonly tags?: SpaceInstallation["tags"];
}

/** Install a package release, binding its declarations to this space's resources and secrets. */
export function install<const Handle extends PackageHandle>(
    handle: Handle,
    bindings: InstallBindingMap<Handle>,
    options: InstallOptions = {},
): SpaceInstallation {
    // key bindings by each declaration's own package
    const bound: SpaceInstallation["bindings"] = {};
    for (const [name, selection] of Object.entries(bindings) as [string, SecretSelection][]) {
        const resource = handle.resources[name];
        const secret = handle.secrets[name];
        // bind a resource declaration to a resource of the stack, with the state it requires
        if (resource && typeof selection === "string") {
            bound[resource.package.id] ??= {};
            bound[resource.package.id][resource.name] = {
                target: { type: "resource", name: selection },
                state: resource.state(),
            };
        }
        // bind a secret declaration to a secret of the stack, pinned to a version when selected
        else if (secret) {
            const target = typeof selection === "string" ? { secret: selection } : selection;
            bound[secret.package.id] ??= {};
            bound[secret.package.id][secret.name] = {
                target: { type: "secret", name: target.secret },
                ...("version" in target ? { version: target.version } : {}),
                state: {},
            };
        }
        // refuse a name the package declares nothing under
        else {
            throw new SpaceError(
                "INVALID_DEFINITION",
                `${handle.name} declares no ${name} to bind`,
            );
        }
    }

    return SpaceInstallation.parse({
        package: { id: handle.id, name: handle.name, version: handle.version },
        status: options.status ?? "enabled",
        alias: options.alias ?? handle.name.slice(handle.name.lastIndexOf("/") + 1),
        bindings: bound,
        compute: options.compute ?? {},
        tags: options.tags ?? {},
        ...(options.policies ? { policies: options.policies } : {}),
    });
}
