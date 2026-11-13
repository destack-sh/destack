import type { Crypto } from "@destack/environment";
import {
    OBJECT_KIND,
    ObjectDescription,
    type ObjectType,
    PACKAGE_KIND,
    PACKAGE_EXPORT,
    type Package,
    type PackageIdentity,
} from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import { graph, type PackageVersion } from "@destack/package";
import {
    type BuildReader,
    PackageManifest,
    type ViewDescription,
    type ViewRender,
} from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceDeclaration } from "@destack/resource";
import type { JsonObject } from "@destack/schema";
import { SecretDeclaration } from "@destack/vault/declare";
import { TestVault } from "@destack/vault/test";
import { VIEW_KIND } from "@destack/view/declare";

/** The public origin a written static view declares for its pages. */
const SITE_ORIGIN = "https://example.test";

/** The main module a view build's browser output exports, as a compiled view's index. */
const BROWSER_MAIN = "browser/index.js";

/** In-memory builds of view packages, whose files the gateway's tests serve. */
export const TestViewBuild = {
    /** Write a package's build in memory with a browser output that mounts views. */
    async of(
        built: PackageVersion,
        views: Readonly<Record<string, ViewDescription>>,
        options: {
            /** The build's other files, by path. */
            readonly files?: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
            /** The package's definition, declared as its build would, with its object types. */
            readonly package?: Package & PackageIdentity;
            /** The object types the package declares, its definition's when absent. */
            readonly objects?: readonly ObjectType[];
            /** The builds of the Destack packages it depends on, which it requires and imports. */
            readonly dependencies?: readonly BuildReader[];
            /** The other declarations it makes, such as commands, each at an export. */
            readonly declarations?: readonly Omit<graph.Declaration, "moniker" | "symbol">[];
        },
        crypto: Crypto,
    ): Promise<BuildReader> {
        // write its main module and each view's beside the other files
        const module = new TextEncoder().encode("export {};\n");
        const files = new Map(options.files);
        files.set(BROWSER_MAIN, module);
        for (const view of Object.values(views)) {
            files.set(view.entrypoint, module);
        }

        // declare the package, its object types, its resources and its views, as its build would
        const objects = options.objects ?? Object.values(options.package?.objects ?? {});
        const declared = [
            ...(options.package === undefined
                ? []
                : [
                      {
                          ...PACKAGE_KIND.identity,
                          name: PACKAGE_EXPORT,
                          description: describePackage(options.package),
                      },
                  ]),
            ...objects.map((type) => ({
                ...OBJECT_KIND.identity,
                name: type.name,
                description: ObjectDescription.of(type),
            })),
            ...Object.values(options.package?.resources ?? {})
                .filter((value) => ResourceDeclaration.is(value))
                .map((resource) => ({
                    ...resource.kind.identity,
                    provisioned: true as const,
                    name: resource.name,
                    description: { ...resource.describe(), ...resource.state() },
                })),
            ...TestVault.declarations(
                [],
                Object.values(options.package?.secrets ?? {}).filter(
                    (value) => value instanceof SecretDeclaration,
                ),
            ),
            ...Object.entries(views).map(([name, view]) => ({
                ...VIEW_KIND.identity,
                name,
                description: {
                    name,
                    presents: view.presents,
                    categories: view.categories,
                    audience: view.audience,
                    packages: view.packages,
                    render: renderOf(view.render),
                },
            })),
            ...(options.declarations ?? []),
        ];

        // require the Destack packages it depends on at their versions
        const given = options.dependencies ?? [];
        const direct = given.map((each) => PackageManifest.package(each.manifest));
        const requires = Object.fromEntries(direct.map((each) => [each.name, each.version]));

        // export the main module and each view's, each importing the direct dependencies
        const imports = direct.map((each) => each.name);
        const exports: Record<string, { path: string; imports: string[] }> = {
            ".": { path: BROWSER_MAIN, imports },
        };
        for (const [name, view] of Object.entries(views)) {
            exports[`./${name}`] = { path: view.entrypoint, imports };
        }

        return MemoryBuild.of(built, declared, crypto, {
            files,
            requires,
            outputs: {
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views,
                    directory: "browser",
                    exports,
                    peers: {},
                },
            },
        });
    },
};

/** Describe how a written view's declaration renders, a static one at the test origin. */
function renderOf(render: ViewRender): JsonObject {
    // describe a static view by the test origin, hydrating its pages whole
    if (render.mode === "static") {
        return { mode: "static", origin: SITE_ORIGIN, hydrate: "page" };
    }

    // describe a client or server view by its mode
    return { mode: render.mode };
}
