import { Caller } from "@destack/access";
import { CompilerPool, outputsOf, Workspace } from "@destack/build";
import { testStore } from "@destack/build/test";
import { Toolchain } from "@destack/check/toolchain";
import { OriginPath } from "@destack/package";
import { BuildImage, type BuildReader } from "@destack/package/manifest";
import { Digest, present, schema } from "@destack/schema";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import { chromium } from "playwright";
import { FRAME_VIEW, ViewContext } from "../declare/context.ts";

/** The space a test frame's context names, which no frame opens. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The universe a test frame's context names, which no frame reaches. */
const UNIVERSE = "http://universe.test";

/** Build the package, serve its frame as an origin does, and start Chromium. */
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

    // map the package's imports below the build path, builtins at their linked builds
    const base = `${OriginPath.build}/`;
    const map = await BuildImage.imports(image, "browser", base, linked, (digest) =>
        store.get(digest),
    );
    const frame = present(reader.manifest.outputs["browser"]?.frame, "the package's frame");
    const context = {
        installation: FRAME_VIEW,
        space: SPACE,
        account: SPACE,
        residencyHome: SPACE,
        view: FRAME_VIEW,
        build: image.root,
        base: "",
        user: Caller.anonymous().subject,
        universe: UNIVERSE,
        catalogs: [],
    } satisfies ViewContext;

    // answer the frame's document, the package's files, and its dependencies' files below the build path
    const server = testHost().server.http({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request) {
            const { pathname } = new URL(request.url);
            // answer the frame's document
            if (pathname === OriginPath.frame) {
                return ViewContext.document(context, frame.entrypoint, frame.stylesheets, map);
            }
            // answer a dependency's file below the build path
            else if (pathname.startsWith(base)) {
                const [digest = "", ...path] = pathname.slice(base.length).split("/");
                return await serve(await store.contents(Digest.parse(digest)), path.join("/"));
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
        root,
    });

    return async () => started.disposeAsync();
}

/** Answer a build's file with the media type the build lists for it. */
async function serve(build: BuildReader, path: string): Promise<Response> {
    const listed = await build.file(path);
    const headers = { "content-type": listed.mediaType };

    return new Response(await build.load(path), { headers });
}
