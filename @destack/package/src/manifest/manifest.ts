import { canonicalize, Digest, Instant, schema, Version } from "@destack/schema";
import {
    DeclarationName,
    DependencyName,
    Language,
    NpmPackage,
    PackageVersion,
    PackageRole,
} from "../definition/index.ts";
import { PackageError } from "../error/error.ts";
import { PackageFile } from "../file/file.ts";
import { PackageOutput } from "./output.ts";

/** The manifest format a build writes. */
export const MANIFEST_FORMAT = 4;

/** The directory of a build's catalogs, which every runtime loads. */
export const CATALOG_DIRECTORY = "locale";

/** The list files of a build, each read on demand and verified by its digest. */
export const ManifestLists = schema.object({
    /** The source, executable and asset files. */
    files: PackageFile,
    /** The source maps of generated files. */
    sourceMaps: PackageFile,
    /** The declarations the build's evaluation described. */
    declarations: PackageFile,
});
/** The list files of a build. */
export type ManifestLists = schema.Infer<typeof ManifestLists>;

/** The portable structure of a package build manifest. */
const manifest = schema.object({
    /** The manifest format version, independent of the package version. */
    formatVersion: schema.literal(MANIFEST_FORMAT),
    /** The package being built: a Destack package, or an npm release built as a unit. */
    package: schema.union([PackageVersion, NpmPackage]),
    /** What the package is, a library for an npm release. */
    role: PackageRole,
    /** The Destack version whose toolchain built it, and whose builtins it links. */
    destack: Version,
    /** The package's source language. */
    language: Language,
    /** The Destack and npm packages it requires, by name, at their declared ranges. */
    requires: schema.record(DependencyName, schema.string().min(1)),
    /** The list files of the build's files, source maps and declarations. */
    lists: ManifestLists,
    /** The digest of what dependents' compilations read of the build, which their keys include. */
    interface: Digest,
    /** Named outputs compiled from the package. */
    outputs: schema.record(DeclarationName, PackageOutput),
});

/** The portable structure of a package build manifest. */
export const PackageManifest = Object.assign(manifest, {
    /** Report whether a build builds a Destack package, not an npm unit. */
    isPackage(
        value: PackageManifest,
    ): value is PackageManifest & { readonly package: PackageVersion } {
        return "id" in value.package;
    },

    /** Read the Destack package a build builds, refusing an npm unit's build. */
    package(value: PackageManifest): PackageVersion {
        // refuse an npm unit's build, which names no Destack package
        if (!PackageManifest.isPackage(value)) {
            const { name, version } = value.package;
            throw PackageError.of(
                "BUILD_INVALID",
                `'${name}@${version}' is invalid: it is an npm package build, not a Destack package build`,
            );
        }

        return value.package;
    },

    /** Serialize a manifest as its canonical bytes, whose digest identifies the build. */
    serialize(value: PackageManifest): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(canonicalize(value));
    },

    /** Read a manifest's bytes from a store that verified their digest, as its build wrote them. */
    deserialize(bytes: Uint8Array<ArrayBuffer>): PackageManifest {
        const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);

        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- the verified digest fixes the bytes a parsed manifest wrote
        return JSON.parse(text) as PackageManifest;
    },
});

/** A package build description. */
export type PackageManifest = schema.Infer<typeof manifest>;

/** An immutable package's retrieval endpoint and manifest digest. */
export const PackageLocation = schema.object({
    /** Digest of the exact root manifest bytes. */
    manifest: Digest,
    /** Base URL serving this package's manifest, files and archive. */
    url: schema.url(),
    /** The expiry time in Unix milliseconds, absent for retained packages. */
    expiresAt: Instant.exactOptional(),
});
/** An immutable package's retrieval endpoint and manifest digest. */
export type PackageLocation = schema.Infer<typeof PackageLocation>;
