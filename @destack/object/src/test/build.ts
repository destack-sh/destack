import type { Crypto } from "@destack/environment";
import { PackageVersion } from "@destack/package";
import { type BuildReader, type OutputContent, PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceDeclaration } from "@destack/resource";
import { PACKAGE_KIND, PACKAGE_EXPORT, type Package, SERVER_EXPORT } from "../declare/index.ts";
import { describePackage } from "../inspect/index.ts";
import { OBJECT_KIND, ObjectDescription, type ObjectType } from "../object/index.ts";

/** The output of every test build: a Bun server output exporting the server, running nothing. */
const SERVER_OUTPUT: OutputContent = {
    runtime: "bun",
    emit: true,
    workloads: {},
    views: {},
    directory: "server",
    exports: { [SERVER_EXPORT]: { path: "server/index.js", imports: [] } },
};

/** A declaration a test build declares beside its package and object types. */
export type TestDeclaration = Parameters<typeof MemoryBuild.of>[1][number];

/** What a test build has beside its package and object types. */
export type TestBuildOptions = Omit<
    NonNullable<Parameters<typeof MemoryBuild.of>[3]>,
    "requires"
> & {
    /** The further declarations, such as settings or notifications, none by default. */
    readonly declared?: readonly TestDeclaration[];
    /** The builds of the packages it requires, which its types may refer to, none by default. */
    readonly dependencies?: readonly BuildReader[];
};

/** Write a package's build as its compiler does: its package, object types and resources. */
export async function testBuild(
    handle: Package & { readonly objects: Readonly<Record<string, ObjectType>> },
    crypto: Crypto,
    options: TestBuildOptions = {},
): Promise<BuildReader> {
    // declare the package, each of its object types and resources, as every installable package's build does
    const { declared = [], dependencies = [], ...contents } = options;
    const described: TestDeclaration[] = [
        {
            ...PACKAGE_KIND.identity,
            name: PACKAGE_EXPORT,
            description: describePackage(handle),
        },
        ...Object.values(handle.objects).map((type) => ({
            ...OBJECT_KIND.identity,
            name: type.name,
            description: ObjectDescription.of(type),
        })),
        ...Object.values(handle.resources)
            .filter((resource) => ResourceDeclaration.is(resource))
            .map((resource) => ({
                ...resource.kind.identity,
                provisioned: true as const,
                name: resource.name,
                description: { ...resource.describe(), ...resource.state() },
            })),
    ];

    // require each dependency at its version
    const requires = Object.fromEntries(
        dependencies.map((dependency) => {
            const owner = PackageManifest.package(dependency.manifest);

            return [owner.name, owner.version] as const;
        }),
    );

    // write the build with a server output unless given others
    return MemoryBuild.of(PackageVersion.of(handle), [...described, ...declared], crypto, {
        outputs: { server: SERVER_OUTPUT },
        ...contents,
        requires,
    });
}
