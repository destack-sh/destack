import {
    CapabilityName,
    ComputeDefinition,
    DeclarationName,
    DirectoryGrant,
    Package,
    PackageId,
} from "@destack/package";
import { InstallationName } from "@destack/runtime";
import { PackagePath } from "@destack/package/file";
import { defineSchema, Digest, schema, Version } from "@destack/schema";
import { RESOURCE_TARGET, SECRET_TARGET, StackBinding } from "./binding.ts";
import type { PackageHandle } from "@destack/runtime/declare";
import { StackError } from "../error/index.ts";

/** A package directory within a repository or checkout, or its root. */
export const PackageDirectory = schema.union([schema.literal("."), PackagePath]);

/** A published registry release by version. */
const Release = schema.object({
    /** A published registry release. */
    kind: schema.literal("release"),
    /** The released package version. */
    version: Version,
});

/** The release a distribution tag points at. */
const Tag = schema.object({
    /** The release a distribution tag points at. */
    kind: schema.literal("tag"),
    /** The distribution tag, such as latest. */
    tag: schema.string().min(1),
});

/** A release by version, or the one a distribution tag points at now. */
export type ReleaseReference = schema.Infer<typeof Release> | schema.Infer<typeof Tag>;

/** The release, repository reference or checkout an installation follows. */
export const InstallationSelection = defineSchema(
    schema.discriminatedUnion("kind", [
        Release,
        Tag,
        schema.object({
            /** Follow committed repository history. */
            kind: schema.literal("repository"),
            /** The registered repository. */
            repository: schema.identifier("repository"),
            /** The full branch or tag reference. */
            reference: schema.string().regex(/^refs\/(heads|tags)\/.+$/u),
            /** The package directory within the repository. */
            directory: PackageDirectory,
        }),
        schema.object({
            /** Follow a local working directory, including unpublished changes. */
            kind: schema.literal("checkout"),
            /** The machine administering the checkout. */
            machine: schema.identifier("machine"),
            /** The checkout registered on that machine. */
            checkout: schema.identifier("checkout"),
            /** The package directory within the checkout. */
            directory: PackageDirectory,
        }),
    ]),
);
/** The release, repository reference or checkout an installation follows. */
export type InstallationSelection = schema.Infer<typeof InstallationSelection>;

/** The exact build a revision evaluated: a registry release, or a build of the space. */
export const InstallationBuild = defineSchema(
    schema.discriminatedUnion("kind", [
        Release.extend({
            /** The digest of the release manifest. */
            manifest: Digest,
        }),
        schema.object({
            /** A succeeded build of the space. */
            kind: schema.literal("build"),
            /** The build. */
            build: schema.identifier("build"),
            /** The digest of the build's manifest. */
            manifest: Digest,
        }),
    ]),
);
/** The exact build a revision evaluated. */
export type InstallationBuild = schema.Infer<typeof InstallationBuild>;

/** The build a submission selects. */
export const BuildReference = defineSchema(
    schema.discriminatedUnion("kind", [
        Release.extend({
            /** The digest of the release manifest, where the submitter pinned it. */
            manifest: Digest.exactOptional(),
        }),
        Tag,
        schema.object({
            /** A succeeded build of the space. */
            kind: schema.literal("build"),
            /** The build. */
            build: schema.identifier("build"),
        }),
    ]),
);
/** The build a submission selects. */
export type BuildReference = schema.Infer<typeof BuildReference>;

/** Select an installation by configuration key or persistent identifier in the space. */
export const InstallationReference = defineSchema(
    schema.union([
        DeclarationName,
        schema.object({
            /** The existing installation checked when applying the configuration. */
            id: schema.identifier("installation"),
        }),
    ]),
);

/** A package installation declared in a configuration. */
export const InstallationDefinition = defineSchema(
    schema.object({
        /** The package release selected by this configuration. */
        package: Package.extend({
            /** The digest of the release manifest, absent to resolve the version at the forge. */
            manifest: Digest.exactOptional(),
        }),
        /** Whether this installation should serve requests. */
        status: schema.enum(["enabled", "suspended"]),
        /** Space-local URL name, independent of the stable installation key. */
        name: InstallationName,
        /** The bindings of the package's declarations, by package ID and declaration name. */
        bindings: schema.record(PackageId, schema.record(DeclarationName, StackBinding)),
        /** Workload compute settings checked against package and machine policy. */
        compute: schema.record(DeclarationName, ComputeDefinition),
        /** The optional capabilities the installation allows. */
        capabilities: schema.array(CapabilityName),
        /** The directories granted to the package's `fs` capability, each with its access. */
        directories: schema.array(DirectoryGrant),
        /** User-defined labels. */
        tags: schema.record(schema.string().min(1), schema.string()),
    }),
);
/** A package installation declared in a configuration. */
export type InstallationDefinition = schema.Infer<typeof InstallationDefinition>;

/** A space secret selected for a package secret, optionally pinned to one version. */
export type SecretSelection = string | { readonly secret: string; readonly version: number };

/** Space resource and secret keys selected for a package's declarations. */
export type InstallBindingMap<Handle extends Pick<PackageHandle, "resources" | "secrets">> = {
    readonly [Name in keyof Handle["resources"]]?: string;
} & { readonly [Name in keyof Handle["secrets"]]?: SecretSelection };

/** Installation settings other than bindings. */
export interface InstallOptions {
    /** Space-local URL name, the package name without its scope by default. */
    readonly name?: string;
    /** Whether the installation serves requests, enabled by default. */
    readonly status?: InstallationDefinition["status"];
    /** Workload compute settings. */
    readonly compute?: InstallationDefinition["compute"];
    /** The optional capabilities allowed, none by default. */
    readonly capabilities?: readonly CapabilityName[];
    /** The directories granted to the package's `fs` capability, none by default. */
    readonly directories?: readonly DirectoryGrant[];
    /** User-defined labels. */
    readonly tags?: InstallationDefinition["tags"];
}

/** Install a package release, binding its declarations to this space's resources and secrets. */
export function install<
    const Handle extends Pick<PackageHandle, "id" | "name" | "version" | "resources" | "secrets">,
>(
    handle: Handle,
    bindings: InstallBindingMap<Handle>,
    options: InstallOptions = {},
): InstallationDefinition {
    // key bindings by each declaration's own package
    const bound: InstallationDefinition["bindings"] = {};
    const selections: readonly (readonly [string, SecretSelection | undefined])[] =
        Object.entries(bindings);
    for (const [name, selection] of selections) {
        const resource = handle.resources[name];
        const secret = handle.secrets[name];
        // leave a declaration bound to nothing
        if (selection === undefined) {
            continue;
        }

        // bind a resource declaration to a resource of the stack with the state it requires, by its key alone
        if (resource !== undefined) {
            if (typeof selection !== "string") {
                throw new StackError(
                    "INVALID_DEFINITION",
                    `${handle.name} declares ${name} as a resource, bound by its key alone`,
                );
            }
            const packageBindings = (bound[resource.package.id] ??= {});
            packageBindings[resource.name] = {
                target: { type: RESOURCE_TARGET, name: selection },
                state: resource.state(),
            };
        }
        // bind a secret declaration to a secret of the stack pinned to a version when selected
        else if (secret !== undefined) {
            const target =
                typeof selection === "string"
                    ? { secret: selection }
                    : { secret: selection.secret, version: selection.version };
            const packageBindings = (bound[secret.package.id] ??= {});
            packageBindings[secret.name] = {
                target: { type: SECRET_TARGET, name: target.secret },
                ...("version" in target ? { version: target.version } : {}),
                state: {},
            };
        }
        // refuse a name the package declares nothing under
        else {
            throw new StackError(
                "INVALID_DEFINITION",
                `${handle.name} declares no ${name} to bind`,
            );
        }
    }

    return InstallationDefinition.parse({
        package: { id: handle.id, name: handle.name, version: handle.version },
        status: options.status ?? "enabled",
        name: options.name ?? handle.name.slice(handle.name.lastIndexOf("/") + 1),
        bindings: bound,
        compute: options.compute ?? {},
        tags: options.tags ?? {},
        capabilities: options.capabilities ?? [],
        directories: options.directories ?? [],
    });
}
