import { DeclarationName, PackageId } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { schema, Version } from "@destack/schema";
import { AccessName } from "@destack/sync";
import { InstallationName } from "./name.ts";

/** A package directory within a repository or checkout, or its root. */
export const PackageDirectory = schema.union([schema.literal("."), PackagePath]);

/** A published registry release by version, immutable once published. */
export const Release = schema.object({
    /** A published registry release. */
    kind: schema.literal("release"),
    /** The released package version. */
    version: Version,
});

/** The release a distribution tag points at. */
export const Tag = schema.object({
    /** The release a distribution tag points at. */
    kind: schema.literal("tag"),
    /** The distribution tag, such as latest. */
    tag: schema.string().min(1),
});

/** The release its host provides, which moves as the host upgrades. */
export const Host = schema.object({
    /** Follow the release the host provides. */
    kind: schema.literal("host"),
});

/** The release an installation runs: by version, by a tag, or as its host provides it. */
export const ReleaseVersion = schema.discriminatedUnion("kind", [Release, Tag, Host]);
/** The release an installation runs. */
export type ReleaseVersion = schema.Infer<typeof ReleaseVersion>;

/** The schema of the version an installation runs. */
const versions = schema.discriminatedUnion("kind", [
    Release,
    Tag,
    Host,
    schema.object({
        /** Follow committed repository history. */
        kind: schema.literal("repository"),
        /** The registered repository. */
        repository: schema.identifier("repository"),
        /** The full branch, tag or checkout reference. */
        reference: schema.string().regex(/^refs\/(heads|tags|checkouts)\/.+$/u),
        /** The package directory within the repository. */
        directory: PackageDirectory,
    }),
]);
/** The version an installation runs: a release, a tag, the host's, or a repository reference. */
export const InstallationVersion = Object.assign(versions, {
    /** Report whether a version resolves to a release: by version, by tag, or the host's. */
    isRelease(version: InstallationVersion): boolean {
        return version.kind === "release" || version.kind === "tag" || version.kind === "host";
    },
});
/** The version an installation runs. */
export type InstallationVersion = schema.Infer<typeof versions>;
/** An installation's version at a repository reference. */
export type RepositoryVersion = Extract<InstallationVersion, { readonly kind: "repository" }>;

/** A bindable object a binding targets. */
export const BindingTarget = schema.union([
    schema.object({
        /** The bindable kind: a resource kind such as database, or a served kind such as secret. */
        type: AccessName,
        /** The object's key in this configuration. */
        name: DeclarationName,
    }),
    schema.object({
        /** The bindable kind: a resource kind such as database, or a served kind such as secret. */
        type: AccessName,
        /** The existing object in the destination space. */
        id: schema.anyIdentifier(),
    }),
    schema.object({
        /** An installation of another space. */
        type: schema.literal("installation"),
        /** The installation's name in that space. */
        name: InstallationName,
        /** The other space, which grants the binding. */
        spaceId: schema.identifier("space"),
    }),
]);
/** A bindable object a binding targets. */
export type BindingTarget = schema.Infer<typeof BindingTarget>;

/** The target an installation selects for one package declaration. */
export const BindingSelection = schema.object({
    /** The bound object. */
    target: BindingTarget,
    /** An exact version or generation to run with, else the target's current one. */
    version: schema.number().int().positive().exactOptional(),
});
/** The target an installation selects for one package declaration. */
export type BindingSelection = schema.Infer<typeof BindingSelection>;

/** The targets an installation selects for its package's declarations. */
export const InstallationBindings = schema.record(
    PackageId,
    schema.record(DeclarationName, BindingSelection),
);
/** The targets an installation selects for its package's declarations. */
export type InstallationBindings = schema.Infer<typeof InstallationBindings>;
