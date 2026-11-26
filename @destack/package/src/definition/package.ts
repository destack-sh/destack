import { nth, Digest, schema, Version } from "@destack/schema";

/** The pattern of a package-local name: lowercase words and hyphens, such as role-permission. */
export const NAME_PATTERN = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u;

/** A concrete package export containing runnable code. */
export const Entrypoint = schema.string().regex(/^\.(?:\/[^\s*]+)?$/u);

/** The URI scheme prefix of a package as an OAuth resource (RFC 8707 asks an absolute URI). */
const RESOURCE_PREFIX = "urn:destack:package:";

/** The schema of a package's identifier. */
const packageIdentifier = schema.identifier("package");

/** The immutable identity retained across package renames and releases. */
export const PackageId = Object.assign(packageIdentifier, {
    /** Name a package as the OAuth resource its access tokens are for, their `aud`. */
    resource(id: PackageId): string {
        return `${RESOURCE_PREFIX}${id}`;
    },
    /** Read the package an OAuth resource names, refusing any other resource. */
    ofResource(resource: string): schema.Infer<typeof packageIdentifier> {
        if (!resource.startsWith(RESOURCE_PREFIX)) {
            throw new TypeError(`resource '${resource}' names no package`);
        }

        return packageIdentifier.parse(resource.slice(RESOURCE_PREFIX.length));
    },
});
/** The immutable identity retained across package renames and releases. */
export type PackageId = schema.Infer<typeof PackageId>;

/** The pattern of a dotted declaration name: camelCase words and dots, such as editor.mode. */
const DOTTED_PATTERN = /^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$/u;

/** The pattern of a member of a qualified name: one camelCase run, such as historyUnavailable. */
const MEMBER_PATTERN = /^[a-z][a-zA-Z0-9]*$/u;

/** The schema of a declaration name within a package. */
const declarationName = schema
    .string()
    .regex(
        new RegExp(
            `^(?:${NAME_PATTERN.source.slice(1, -1)}|${DOTTED_PATTERN.source.slice(1, -1)})$`,
            "u",
        ),
    );

/** A declaration name within a package: kebab-case, or dotted camelCase such as editor.mode. */
export const DeclarationName = Object.assign(declarationName, {
    /** Qualify a member by the identifier of what it shows, as `<shown>.<member>` in camelCase. */
    qualify(shown: string, member: string): schema.Infer<typeof declarationName> {
        // require a camelCase member
        if (!MEMBER_PATTERN.test(member)) {
            throw new TypeError(
                `'${member}' is invalid: it must be camelCase, such as historyUnavailable`,
            );
        }

        // write the shown identifier in camelCase: Tabs as tabs, notesBoard as it is
        const owner = `${shown.charAt(0).toLowerCase()}${shown.slice(1)}`;

        // require a valid name, refusing it with the shown identifier it came from
        const qualified = `${owner}.${member}`;
        if (!declarationName.safeParse(qualified).success) {
            throw new TypeError(
                `'${qualified}' is invalid: it shows '${shown}', whose name is no declaration name`,
            );
        }

        return qualified;
    },
});
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

/** An account's Git repository name, a DNS label of at most 63 characters (RFC 1035 2.3.4). */
export const RepositoryName = schema.string().regex(/^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/u);
/** An account's Git repository name. */
export type RepositoryName = schema.Infer<typeof RepositoryName>;

/** A peer dependency a package declares, which its dependents provide. */
export const PeerDependency = schema.object({
    /** Whether a dependent may leave it out, as npm's peerDependenciesMeta marks it. */
    isOptional: schema.boolean(),
});
/** A peer dependency a package declares. */
export type PeerDependency = schema.Infer<typeof PeerDependency>;

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

        /** Read the export subpath of a bare import specifier: `.`, or `./` and the rest. */
        subpath(specifier: string): string {
            const rest = specifier.slice(DependencyName.of(specifier).length);

            return `.${rest}`;
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
export const PackageVersion = Object.assign(packageSchema, {
    /** Read the identity of a value with a package's, such as its handle. */
    of(value: PackageVersion): PackageVersion {
        const { id, name, version } = value;

        return { id, name, version };
    },

    /** Read the package a declaration has, under the package key or its `package` property. */
    declaring(value: unknown): PackageVersion | undefined {
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
export type PackageVersion = Readonly<schema.Infer<typeof packageSchema>>;

/** A Destack package's build: the package and the digest of its build's manifest. */
export const PackageBuild = schema.object({
    /** The package name and version. */
    package: PackageVersion,
    /** The digest of its build's manifest. */
    manifest: Digest,
});
/** A Destack package's build. */
export type PackageBuild = schema.Infer<typeof PackageBuild>;

/** The structure of a package release. */
const packageRelease = schema.object({
    /** The released package name and version. */
    package: PackageVersion,
    /** The digest of its image: its build and every build it runs with. */
    image: Digest,
});

/** An immutable Destack release and the image it runs with. */
export const PackageRelease = Object.assign(packageRelease, {
    /** Select the newest of the releases of a package, none when it has none. */
    newest(releases: readonly PackageRelease[], packageId: PackageId): PackageRelease | undefined {
        return releases
            .filter((each) => each.package.id === packageId)
            .toSorted((left, right) =>
                Version.compare(
                    Version.parse(right.package.version),
                    Version.parse(left.package.version),
                ),
            )[0];
    },

    /** Select a package's release at a version. */
    select(
        releases: readonly PackageRelease[],
        packageId: PackageId,
        version: string,
    ): PackageRelease | undefined {
        return releases.find(
            (each) => each.package.id === packageId && each.package.version === version,
        );
    },
});
/** An immutable Destack package release. */
export type PackageRelease = schema.Infer<typeof packageRelease>;
