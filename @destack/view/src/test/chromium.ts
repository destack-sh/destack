import { Caller } from "@destack/access";
import { CompilerPool, outputsOf, Workspace } from "@destack/build";
import { testStore } from "@destack/build/test";
import { Toolchain } from "@destack/check/toolchain";
import { Catalog } from "@destack/locale";
import { OriginPath } from "@destack/package";
import {
    BuildImage,
    type BuildReader,
    MODULE_TYPE,
    ModuleResolution,
    PackageManifest,
} from "@destack/package/manifest";
import { present, schema } from "@destack/schema";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import { chromium } from "playwright";
import { EXAMPLE_VIEW, ViewContext } from "../declare/context.ts";

/** The space a test example page's context names, which no page opens. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The universe a test example page's context names, which no page reaches. */
const UNIVERSE = "http://universe.test";

/** Build the package, serve its example page as an origin does, and start Chromium. */
export default async function setup(project: TestProject): Promise<() => Promise<void>> {
    // build every output of the package from its workspace, as `destack dev` does
    const root = project.config.root;
    const started = new AsyncDisposableStack();
    const store = started.use(await testStore());
    const compilers = started.use(
        await CompilerPool.open(store, async () => Toolchain.workspace()),
    );
    const workspace = await (await Workspace.open(root, compilers)).package();
    const reader = await workspace.build({
        outputs: outputsOf(workspace.root.packageJson),
        isUnreleased: true,
    });

    // build each builtin member into the image a machine links for it, then the package's image
    const linked = new Map<string, BuildImage>();
    for (const unit of workspace.units.filter(
        (each) => each.packageJson.definition.role === "builtin",
    )) {
        const member = await (await Workspace.open(unit.directory, compilers)).package();
        const built = await member.build({
            outputs: outputsOf(unit.packageJson),
            isUnreleased: true,
        });
        linked.set(unit.name, await store.image(await member.image(await built.digest())));
    }
    const image = await store.image(await workspace.image(await reader.digest()));

    // resolve the package's modules, builtins at their linked builds
    const base = `${OriginPath.build}/`;
    const map = await BuildImage.imports(image, "browser", base, linked, (digest) =>
        store.get(digest),
    );
    const host = testHost();
    const resolution = await ModuleResolution.of(
        map,
        image.root,
        { builds: base, modules: `${OriginPath.module}/` },
        async (build) => PackageManifest.dependencies(await store.get(build), "browser"),
        host.crypto,
    );
    const example = present(
        reader.manifest.outputs["browser"]?.example,
        "the package's example page",
    );
    const context = {
        installation: EXAMPLE_VIEW,
        space: SPACE,
        account: SPACE,
        view: EXAMPLE_VIEW,
        build: image.root,
        base: "",
        user: Caller.anonymous().subject,
        universe: UNIVERSE,
        catalogs: (await reader.files())
            .map((file) => file.path)
            .filter((path) => Catalog.isPath(path)),
    } satisfies ViewContext;

    // answer the example page, the modules and the package's files
    const server = host.server.http({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request) {
            const { pathname } = new URL(request.url);
            // answer the example page's document
            if (pathname === OriginPath.example) {
                const entry = resolution.url(image.root, example.entrypoint);
                return ViewContext.document(context, entry, example.stylesheets, []);
            }
            // answer a module
            else if (pathname.startsWith(resolution.prefixes.modules)) {
                const located = present(resolution.locate(pathname), `module '${pathname}'`);
                const build = await store.contents(located.build);
                const served = await serve(build, located.path);
                return served.headers.get("content-type") === MODULE_TYPE
                    ? new Response(await resolution.rewrite(await served.text(), pathname), served)
                    : served;
            }
            // answer a file of the package
            else {
                return await serve(reader, pathname.slice(1));
            }
        },
    });
    started.defer(async () => server.stop(true));

    // launch the Chromium every scenario of the run connects to
    const browser = await chromium.launchServer({ headless: true });
    started.defer(async () => browser.close());
    project.provide("chromium", {
        endpoint: browser.wsEndpoint(),
        origin: server.url.origin,
    });

    return async () => started.disposeAsync();
}

/** Answer a build's file with the media type the build lists for it. */
async function serve(build: BuildReader, path: string): Promise<Response> {
    const listed = await build.file(path);
    const headers = { "content-type": listed.mediaType };

    return new Response(await build.load(path), { headers });
}
