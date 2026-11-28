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
    type PackageOutput,
    type ViewDescription,
    type ViewRender,
} from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceDeclaration } from "@destack/resource";
import type { JsonObject } from "@destack/schema";
import { SecretDeclaration } from "@destack/vault/declare";
import { TestVault } from "@destack/vault/test";
import { VIEW_KIND, VIEW_WORKLOAD } from "@destack/view/declare";
import { KNOWN_TYPES } from "@destack/stack/stack";
import { describedTables } from "@destack/object/test";

/** The public origin a written static view declares for its pages. */
const SITE_ORIGIN = "https://example.test";

/** The main module a view build's browser output exports, as a compiled view's index. */
const BROWSER_MAIN = "browser/index.js";

/** The module a view build's server output runs its view workload from. */
const VIEWS_MODULE = `server/workload/${VIEW_WORKLOAD}.js`;

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
            /** The example page of a development build, which its origin serves. */
            readonly example?: PackageOutput["example"];
        },
        crypto: Crypto,
    ): Promise<BuildReader> {
        // add empty entries the files lack
        const module = new TextEncoder().encode("export {};\n");
        const files = new Map(options.files);
        const entries = [
            BROWSER_MAIN,
            ...Object.values(views).map((view) => view.entrypoint),
            ...(options.example === undefined ? [] : [options.example.entrypoint]),
        ];
        for (const entry of entries.filter((path) => !files.has(path))) {
            files.set(entry, module);
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
                    opens: view.opens,
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
        if (options.example !== undefined) {
            exports["./example"] = { path: options.example.entrypoint, imports };
        }

        // render the client views in a server output's view workload, as a view build derives it
        const isRendering = Object.values(views).some((view) => view.render.mode === "client");
        const rendering = isRendering ? TestViewBuild.views(built) : undefined;
        for (const [path, bytes] of rendering?.files ?? []) {
            files.set(path, bytes);
        }

        return MemoryBuild.of(built, declared, crypto, {
            files,
            requires,
            tables: describedTables(declared, built.id, KNOWN_TYPES),
            outputs: {
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views,
                    ...(options.example === undefined ? {} : { example: options.example }),
                    directory: "browser",
                    exports,
                    inlined: {},
                    peers: {},
                },
                ...rendering?.outputs,
            },
        });
    },

    /** Write the view workload a view build derives for client views: its module and output. */
    views(built: PackageVersion): {
        readonly files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
        readonly outputs: Readonly<Record<string, PackageOutput>>;
    } {
        return {
            files: new Map([[VIEWS_MODULE, new TextEncoder().encode(viewsModule(built))]]),
            outputs: {
                server: {
                    runtime: "bun",
                    emit: true,
                    workloads: {
                        [VIEW_WORKLOAD]: {
                            kind: "view",
                            entrypoint: `./workload/${VIEW_WORKLOAD}`,
                            packages: [],
                            resources: [],
                            secrets: [],
                            compute: {},
                            sandbox: {},
                        },
                    },
                    views: {},
                    directory: "server",
                    exports: {
                        [`./workload/${VIEW_WORKLOAD}`]: {
                            path: VIEWS_MODULE,
                            imports: ["@destack/object/bun", "@destack/object/workload"],
                        },
                    },
                    inlined: {},
                    peers: {},
                },
            },
        };
    },
};

/** Write a view workload answering each render with its request's head and context. */
function viewsModule(built: PackageVersion): string {
    const owner = JSON.stringify({ id: built.id, name: built.name, version: built.version });

    return [
        'import { hostWorkload } from "@destack/object/bun";',
        'import { defineWorkload } from "@destack/object/workload";',
        `const owner = ${owner};`,
        "const escape = (url) => url.replaceAll('&', '&amp;').replaceAll('\"', '&quot;').replaceAll('<', '&lt;');",
        "const document = (asked) => [",
        "    '<!doctype html><html><head>',",
        "    `<script type=\"application/json\" id=\"destack-view\">${JSON.stringify(asked.context).replaceAll('<', '\\\\u003c')}</script>`,",
        '    ...asked.head.stylesheets.map((href) => `<link rel="stylesheet" href="${escape(href)}">`),',
        '    ...asked.head.preloads.map((href) => `<link rel="modulepreload" href="${escape(href)}">`),',
        '    `<script type="module" src="${escape(asked.head.script)}" async></script>`,',
        "    '</head><body></body></html>',",
        "].join('');",
        "const start = () => ({ packages: [], view: { fetch: async (request) => new Response(document(await request.json()), { headers: { 'content-type': 'text/html; charset=utf-8' } }) } });",
        `const workload = defineWorkload({ name: ${JSON.stringify(VIEW_WORKLOAD)}, start }, { package: owner });`,
        "export default workload;",
        "export const Workload = hostWorkload(workload, {});",
        "if (import.meta.main) await Workload.run();",
    ].join("\n");
}

/** Describe how a written view's declaration renders, a static one at the test origin. */
function renderOf(render: ViewRender): JsonObject {
    // describe a static view by the test origin, hydrating its pages whole
    if (render.mode === "static") {
        return { mode: "static", origin: SITE_ORIGIN, hydrate: "page" };
    }

    // describe a client or server view by its mode
    return { mode: render.mode };
}
