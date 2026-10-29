import { DeclarationName, PackageId } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { Digest, schema, Version } from "@destack/schema";
import { AccessName } from "@destack/sync";
import { InstallationName } from "../declare/egress.ts";

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
export const PinnedRelease = Release.extend({
    /** The digest of the release manifest, absent to resolve the version at the registry. */
    manifest: Digest.exactOptional(),
});

/** The release a distribution tag points at. */
export const Tag = schema.object({
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
