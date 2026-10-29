import { ComputeDefinition, DeclarationName, DirectoryGrant, PackageId } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { Digest, schema, Version } from "@destack/schema";
import { SECRET_TARGET, StackBinding } from "./binding.ts";
import { InstallationName, type PackageHandle } from "@destack/object";
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

/** A published release by version, pinned to its manifest where the selector pinned it. */
const PinnedRelease = Release.extend({
    /** The digest of the release manifest, absent to resolve the version at the registry. */
    manifest: Digest.exactOptional(),
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
export const InstallationSelection = schema.discriminatedUnion("kind", [
    PinnedRelease,
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
]);
/** The release, repository reference or checkout an installation follows. */
export type InstallationSelection = schema.Infer<typeof InstallationSelection>;
/** An installation's selection of a repository reference. */
export type RepositorySelection = Extract<InstallationSelection, { readonly kind: "repository" }>;

/** The build a submission selects. */
export const BuildReference = schema.discriminatedUnion("kind", [
    PinnedRelease,
    Tag,
    schema.object({
        /** A succeeded build of the space. */
        kind: schema.literal("build"),
        /** The build. */
        build: schema.identifier("build"),
    }),
]);
/** The build a submission selects. */
export type BuildReference = schema.Infer<typeof BuildReference>;

/** The bindings of a package's declarations, by package ID and declaration name. */
export const InstallationBindings = schema.record(
    PackageId,
    schema.record(DeclarationName, StackBinding),
);
/** The bindings of a package's declarations, by package ID and declaration name. */
export type InstallationBindings = schema.Infer<typeof InstallationBindings>;

/** An application installation as a stack declares it: its create input. */
export const InstallationDefinition = schema.object({
    /** The installation's kind: a package's, since a stack declares no stacks. */
    kind: schema.literal("package"),
    /** The installed package. */
    packageId: PackageId,
    /** The release the installation follows, pinned to its manifest where the stack pinned it. */
    selection: PinnedRelease,
    /** Whether this installation should serve requests. */
    status: schema.enum(["enabled", "suspended"]),
    /** Space-local URL name, independent of the stable installation key. */
    name: InstallationName,
    /** The bindings of the package's declarations, by package ID and declaration name. */
    bindings: InstallationBindings,
    /** Workload compute settings checked against package and machine policy. */
    compute: schema.record(DeclarationName, ComputeDefinition),
    /** The package's optional capabilities the installation allows. */
    capabilities: schema.array(DeclarationName),
    /** The directories granted to the package's `files` capability, each with its access. */
    directories: schema.array(DirectoryGrant),
    /** User-defined labels. */
    tags: schema.record(schema.string().min(1), schema.string()),
});
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
    readonly capabilities?: readonly DeclarationName[];
    /** The directories granted to the package's `files` capability, none by default. */
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

        // bind a resource declaration to a resource of the stack by its key alone
        if (resource !== undefined) {
            if (typeof selection !== "string") {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `${handle.name} declares ${name} as a resource, so bind it by key only`,
                );
            }
            const packageBindings = (bound[resource.package.id] ??= {});
            packageBindings[resource.name] = {
                target: { type: resource.kind, name: selection },
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
            };
        }
        // refuse a name the package declares nothing under
        else {
            throw StackError.of("DEFINITION_INVALID", `${handle.name} declares no ${name} to bind`);
        }
    }

    return InstallationDefinition.parse({
        kind: "package",
        packageId: handle.id,
        selection: { kind: "release", version: handle.version },
        status: options.status ?? "enabled",
        name: options.name ?? handle.name.slice(handle.name.lastIndexOf("/") + 1),
        bindings: bound,
        compute: options.compute ?? {},
        tags: options.tags ?? {},
        capabilities: options.capabilities ?? [],
        directories: options.directories ?? [],
    });
}
