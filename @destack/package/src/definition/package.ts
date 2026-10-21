import { nth, Digest, schema, Version } from "@destack/schema";

/** The pattern of a package-local name: lowercase words and hyphens, such as role-permission. */
export const NAME_PATTERN = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u;

/** A concrete package export containing runnable code. */
export const Entrypoint = schema.string().regex(/^\.(?:\/[^\s*]+)?$/u);

/** The URI scheme prefix of a package as an OAuth resource (RFC 8707 asks an absolute URI). */
const RESOURCE_PREFIX = "urn:destack:package:";

/** The immutable identity retained across package renames and releases. */
export const PackageId = Object.assign(schema.identifier("package"), {
    /** Name a package as the OAuth resource its access tokens are for, their `aud`. */
    resource(id: PackageId): string {
        return `${RESOURCE_PREFIX}${id}`;
    },
});
/** The immutable identity retained across package renames and releases. */
export type PackageId = schema.Infer<typeof PackageId>;

/** The pattern of a dotted declaration name: camelCase words and dots, such as editor.mode. */
const DOTTED_PATTERN = /^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$/u;

/** A declaration name within a package: kebab-case, or dotted camelCase such as editor.mode. */
export const DeclarationName = schema
    .string()
    .regex(
        new RegExp(
            `^(?:${NAME_PATTERN.source.slice(1, -1)}|${DOTTED_PATTERN.source.slice(1, -1)})$`,
            "u",
        ),
    );
/** A declaration name within a package. */
export type DeclarationName = schema.Infer<typeof DeclarationName>;

/** A scoped Destack package name: `@<account handle>/<name>`. */
export const PackageName = Object.assign(
    schema
        .string()
        .max(214)
        .regex(/^@[a-z0-9][a-z0-9._-]*\/[a-z0-9][a-z0-9._-]*$/u),
    {
        /** Read the account handle scoping a package name and the name within the account. */
        split(name: string): { readonly handle: string; readonly local: string } {
            const parts = PackageName.parse(name).slice(1).split("/");

            return { handle: nth(parts, 0), local: nth(parts, 1) };
        },
    },
);

/** A scoped or unscoped dependency name. */
export const DependencyName = Object.assign(
    schema
        .string()
        .max(214)
        .regex(/^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/u),
    {
        /** Read the dependency of a bare import specifier: its scope and name, or its name. */
        of(specifier: string): string {
            return specifier
                .split("/")
                .slice(0, specifier.startsWith("@") ? 2 : 1)
                .join("/");
        },
    },
);

/** A named dependency version, including packages from external registries. */
export const DependencyPackage = schema.object({
    name: DependencyName,
    version: schema.string().min(1),
});
/** A named dependency version. */
export type DependencyPackage = schema.Infer<typeof DependencyPackage>;

/** An npm release as its package.json names it and the lockfile verifies it. */
export const NpmPackage = DependencyPackage.extend({
    /** The registry's Subresource Integrity expression, as the lockfile records it. */
    integrity: schema.string().min(1),
});
/** An npm release as its package.json names it and the lockfile verifies it. */
export type NpmPackage = schema.Infer<typeof NpmPackage>;

/** The key of the declaring package on a declaration with taken properties, such as columns. */
export const PACKAGE = Symbol.for("destack.package");

/** The schema of a package's identity. */
const packageSchema = schema.object({
    /** The identity retained across renames and releases. */
    id: PackageId,
    /** The package name, qualified by its owner. */
    name: PackageName,
    /** The package version. */
    version: Version,
});

/** The immutable identity, current name and version declared by a Destack package. */
export const Package = Object.assign(packageSchema, {
    /** Read the identity of a value with a package's, such as its handle. */
    of(value: Package): Package {
        const { id, name, version } = value;

        return { id, name, version };
    },

    /** Read the package a declaration has, under the package key or its `package` property. */
    declaring(value: unknown): Package | undefined {
        // read no package from a value that is not an object
        if (typeof value !== "object" || value === null) {
            return undefined;
        }
        const declared =
            PACKAGE in value ? value[PACKAGE] : "package" in value ? value.package : undefined;

        return declared === undefined ? undefined : packageSchema.parse(declared);
    },
});

/** A named, versioned package. */
export type Package = Readonly<schema.Infer<typeof packageSchema>>;

/** An immutable Destack release and the manifest identifying its distributed contents. */
export const PackageRelease = schema.object({
    /** The released package name and version. */
    package: Package,
    /** The digest of its immutable build manifest. */
    manifest: Digest,
});
/** An immutable Destack package release. */
export type PackageRelease = schema.Infer<typeof PackageRelease>;
