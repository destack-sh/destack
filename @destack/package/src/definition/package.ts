import { defineSchema, Digest, schema, Version } from "@destack/schema";

/** The pattern of a package-local name: lowercase words joined by single hyphens, such as role-permission. */
export const NAME_PATTERN = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/u;

/** A concrete package export containing runnable code. */
export const Entrypoint = defineSchema(schema.string().regex(/^\.(?:\/[^\s*]+)?$(?![\s\S])/u));

/** The immutable identity retained across package renames and releases. */
export const PackageId = schema.identifier("package");
/** The immutable identity retained across package renames and releases. */
export type PackageId = schema.Infer<typeof PackageId>;

/** A declaration name within a package. */
export const DeclarationName = defineSchema(schema.string().regex(NAME_PATTERN));
/** A declaration name within a package. */
export type DeclarationName = schema.Infer<typeof DeclarationName>;

/** A scoped Destack package name. */
export const PackageName = defineSchema(
    schema
        .string()
        .max(214)
        .regex(/^@[a-z0-9][a-z0-9._-]*\/[a-z0-9][a-z0-9._-]*$(?![\s\S])/u),
);

/** A scoped or unscoped dependency name. */
export const DependencyName = Object.assign(
    defineSchema(
        schema
            .string()
            .max(214)
            .regex(/^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$(?![\s\S])/u),
    ),
    {
        /** Read the dependency a bare import specifier names: its scope and name, or its name. */
        of(specifier: string): string {
            return specifier
                .split("/")
                .slice(0, specifier.startsWith("@") ? 2 : 1)
                .join("/");
        },
    },
);

/** A named dependency version, including packages from external registries. */
export const DependencyPackage = defineSchema(
    schema.object({ name: DependencyName, version: schema.string().min(1) }),
);
/** A named dependency version. */
export type DependencyPackage = schema.Infer<typeof DependencyPackage>;

/** The key of the declaring package on a declaration whose properties are taken, such as a table's columns. */
export const PACKAGE = Symbol.for("destack.package");

/** The schema of a package's identity. */
const packageSchema = defineSchema(
    schema.object({
        /** The identity retained across renames and releases. */
        id: PackageId,
        /** The package name, qualified by its owner. */
        name: PackageName,
        /** The package version. */
        version: Version,
    }),
);

/** The immutable identity, current name and version declared by a Destack package. */
export const Package = Object.assign(packageSchema, {
    /** Read the package a declaration carries, under the package key or its `package` property. */
    declaring(value: unknown): Package | undefined {
        // read no package from a value that is no object
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
export const PackageRelease = defineSchema(
    schema.object({
        /** The released package name and version. */
        package: Package,
        /** The digest of its immutable build manifest. */
        manifest: Digest,
    }),
);
/** An immutable Destack package release. */
export type PackageRelease = schema.Infer<typeof PackageRelease>;
