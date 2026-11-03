import type { Crypto } from "@destack/environment";
import { Package } from "@destack/package";
import { type BuildReader, type OutputContent, PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import {
    PACKAGE_DECLARATION,
    PACKAGE_EXPORT,
    type PackageHandle,
    SERVER_EXPORT,
} from "../declare/index.ts";
import { describePackage } from "../inspect/index.ts";
import { OBJECT_DECLARATION, ObjectDescription, type ObjectType } from "../object/index.ts";
import objectPackage from "../package.ts";

/** The output of every test build: a Bun server output exporting the server, running nothing. */
const SERVER_OUTPUT: OutputContent = {
    runtime: "bun",
    emit: true,
    workloads: {},
    views: {},
    directory: "server",
    exports: { [SERVER_EXPORT]: { path: "server/index.js", imports: [] } },
    dependencies: {},
};

/** A declaration a test build declares beside its package and object types. */
export type TestDeclaration = Parameters<typeof MemoryBuild.of>[1][number];

/** What a test build has beside its package and object types. */
export type TestBuildOptions = Omit<
    NonNullable<Parameters<typeof MemoryBuild.of>[3]>,
    "dependencies"
> & {
    /** The further declarations, such as settings or notifications, none by default. */
    readonly declared?: readonly TestDeclaration[];
    /** The builds of the packages it depends on, which its types may refer to, none by default. */
    readonly dependencies?: readonly BuildReader[];
};

/** Write a package's build as its compiler does, declaring its package and object types. */
export async function testBuild(
    handle: PackageHandle & { readonly objects: Readonly<Record<string, ObjectType>> },
    crypto: Crypto,
    options: TestBuildOptions = {},
): Promise<BuildReader> {
    // declare the package and each of its object types, as every installable package's build does
    const { declared = [], dependencies = [], ...contents } = options;
    const described: TestDeclaration[] = [
        {
            kind: PACKAGE_DECLARATION,
            kindPackageId: objectPackage.id,
            name: PACKAGE_EXPORT,
            description: describePackage(handle),
        },
        ...Object.values(handle.objects).map((type) => ({
            kind: OBJECT_DECLARATION,
            kindPackageId: objectPackage.id,
            name: type.name,
            description: ObjectDescription.of(type),
        })),
    ];

    // record each dependency's build and the closure it compiled against, as a compiler resolves them
    const resolved = await Promise.all(
        dependencies.map(async (dependency) => {
            // take the dependency's release beside the releases it depends on
            const owner = PackageManifest.package(dependency.manifest);
            const manifest = await dependency.digest();
            const release = { kind: "destack" as const, package: owner, manifest };

            return [
                ...Object.entries(await dependency.dependencies()),
                [owner.name, release] as const,
            ];
        }),
    );

    // write the build with a server output unless given others, beside its dependencies
    return MemoryBuild.of(Package.of(handle), [...described, ...declared], crypto, {
        outputs: { server: SERVER_OUTPUT },
        ...contents,
        dependencies: Object.fromEntries(resolved.flat()),
    });
}
