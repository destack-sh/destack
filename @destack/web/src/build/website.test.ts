import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterAll, beforeAll, expect, onTestFinished, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { CompilerPool, type LocalPackageStore, type PackageBuild } from "@destack/build";
import { PackageCopy, testStore, testSimulation } from "@destack/build/test";
import type { Simulation } from "@destack/environment/simulation";
import { requests } from "../../tests/fixture/website/request.ts";

/** The real host, whose files the website fixture lives in. */
const HOST = testHost();

/** The website fixture, an application rendered in each mode. */
const WEBSITE = new URL("../../tests/fixture/website/source/", import.meta.url);

/** The longest a cold build of the website on its stored closure takes. */
const WEBSITE_BUILD_MILLISECONDS = 60_000;

/** The world the tests build in. */
let simulation: Simulation;
/** The checkout's test store. */
let store: LocalPackageStore;
/** The compilers building into the store. */
let compilers: CompilerPool;
/** The website's copy, which every test builds from. */
let source: PackageCopy;

beforeAll(async () => {
    simulation = testSimulation();
    store = await testStore();
    compilers = await CompilerPool.open(store);
    source = await PackageCopy.open(WEBSITE, compilers);
}, WEBSITE_BUILD_MILLISECONDS);

afterAll(async () => {
    await source[Symbol.asyncDispose]();
    await compilers[Symbol.asyncDispose]();
    await store[Symbol.asyncDispose]();
    await simulation[Symbol.asyncDispose]();
});

/** A module whose default export answers requests. */
interface HandlerModule {
    /** The request handler. */
    readonly default: { readonly fetch: (request: Request) => Response | Promise<Response> };
}

/** Report whether a module's default export answers requests. */
function isHandlerModule(module: unknown): module is HandlerModule {
    const handler =
        typeof module === "object" && module !== null && "default" in module
            ? module.default
            : undefined;

    return (
        typeof handler === "object" &&
        handler !== null &&
        "fetch" in handler &&
        typeof handler.fetch === "function"
    );
}

/** List the text of a page's headings. */
function headings(document: string): string[] {
    return [...document.matchAll(/<h1[^>]*>([^<]*)<\/h1>/gu)].map((match) => match[1] ?? "");
}

/** List the root-relative asset references of a page the browser output does not distribute. */
function missingAssets(document: string, distributed: ReadonlySet<string>): string[] {
    return [...document.matchAll(/(?:src|href)="\/([^"]+)"/gu)]
        .map((match) => match[1] ?? "")
        .filter((asset) => !distributed.has(`output/website-browser/${asset}`));
}

/** Read every page a build distributes, by path. */
async function readPages(build: PackageBuild): Promise<Map<string, string>> {
    const pages = new Map<string, string>();
    for (const file of await build.reader.distributed()) {
        if (file.path.endsWith(".html")) {
            pages.set(file.path, new TextDecoder().decode(await build.reader.load(file.path)));
        }
    }

    return pages;
}

test.for(Object.entries(requests))(
    "serve the website's %s build from its placed tree, every page's assets distributed",
    { timeout: WEBSITE_BUILD_MILLISECONDS },
    async ([mode, application]) => {
        // build the website on its stored closure, and place it as a machine does
        await using build = await source.build({ outputs: { website: application } });
        const destination = join(source.directory, `deployed-${mode}`);
        await source.deploy(build, destination);
        const distributed = new Set((await build.reader.distributed()).map((file) => file.path));

        // render the home page from an emitted server output
        const server = build.manifest.outputs["website-server"];
        const entry = server?.emit === true ? server.exports["."]?.path : undefined;
        const rendered =
            entry === undefined ? undefined : await renderHome(join(destination, entry));

        // resolve every page's asset references to distributed files
        const pages = await readPages(build);
        expect({
            rendered:
                rendered === undefined
                    ? undefined
                    : { ...rendered, missing: missingAssets(rendered.document, distributed) },
            missing: Object.fromEntries(
                [...pages].map(([path, page]) => [path, missingAssets(page, distributed)]),
            ),
        }).toEqual({
            rendered:
                rendered === undefined
                    ? undefined
                    : {
                          status: 200,
                          headings: ["Hello Destack"],
                          document: rendered.document,
                          missing: [],
                      },
            missing: Object.fromEntries([...pages.keys()].map((path) => [path, []])),
        });
    },
);

/** Render the home page through a server output's request handler. */
async function renderHome(
    path: string,
): Promise<{ status: number; headings: string[]; document: string }> {
    const module: unknown = await import(pathToFileURL(path).href);
    if (!isHandlerModule(module)) {
        throw new TypeError("the server exports no request handler");
    }
    const response = await module.default.fetch(new Request("https://example.test/"));
    const document = await response.text();

    return { status: response.status, headings: headings(document), document };
}

test(
    "refuse a server entry reading a browser global before framework compilation",
    { timeout: WEBSITE_BUILD_MILLISECONDS },
    async () => {
        // write a server entry reading a browser global, which the website's DOM library types
        const entries = ["server.ts", "client.ts"].map((name) => join(source.source, name));
        onTestFinished(async () => {
            await Promise.all(entries.map((entry) => HOST.files.remove(entry)));
        });
        await HOST.files.writeTextFile(
            join(source.source, "server.ts"),
            "/** Render the page title. */\nexport function render(): string {\n    return document.title;\n}\n",
        );
        await HOST.files.writeTextFile(join(source.source, "client.ts"), "export {};\n");

        // refuse the checked build before framework compilation or rendering
        const application = {
            ...requests.static,
            entryServer: "server.ts",
            entryClient: "client.ts",
        };
        expect(await refusal(source.check({ outputs: { website: application } }))).toEqual([
            "UNPROCESSABLE_CONTENT",
            "RUNTIME_API_UNSUPPORTED",
            "unsupported bun API: document.title at server.ts:76",
        ]);
    },
);

test(
    "rebuild the website with emitted assets and restore every output",
    { timeout: WEBSITE_BUILD_MILLISECONDS },
    async () => {
        // declare the asset imports and retain two distinct emitted assets through the application's render
        const file = join(source.source, "src/app.tsx");
        const original = await HOST.files.readTextFile(file);
        const assets = ["assets.d.ts", "first.svg", "second.svg"].map((name) =>
            join(source.source, "src", name),
        );
        onTestFinished(async () => {
            await HOST.files.writeTextFile(file, original);
            await Promise.all(assets.map((asset) => HOST.files.remove(asset)));
        });
        await HOST.files.writeTextFile(
            join(source.source, "src/assets.d.ts"),
            '/** SVG assets imported as URLs. */\ndeclare module "*.svg?url" {\n    const url: string;\n    export default url;\n}\n',
        );
        for (const name of ["first", "second"]) {
            await HOST.files.writeTextFile(
                join(source.source, `src/${name}.svg`),
                `<svg xmlns="http://www.w3.org/2000/svg"><!--${name.repeat(1500)}--></svg>`,
            );
        }
        const edited =
            'import first from "./first.svg?url";\nimport second from "./second.svg?url";\n' +
            original.replace(
                '<main style={theme.variables({ ...DEFAULT_DISPLAY, appearance: "light" })}>',
                '<main style={theme.variables({ ...DEFAULT_DISPLAY, appearance: "light" })}><img src={first} /><img src={second} />',
            );
        await HOST.files.writeTextFile(file, edited);
        const request = { outputs: { website: requests.browser } };

        // compare the full distribution before and after an application edit
        await using first = await source.build(request);
        await using repeated = await source.build(request);
        await HOST.files.writeTextFile(file, edited.replace("Hello Destack", "Edited Destack"));
        await using changed = await source.build(request);
        await HOST.files.writeTextFile(file, edited);
        await using restored = await source.build(request);
        expect({
            repeated: repeated.manifest,
            repeatedFiles: await repeated.reader.distributed(),
            isChanged: JSON.stringify(changed.manifest) !== JSON.stringify(first.manifest),
            restored: restored.manifest,
            restoredFiles: await restored.reader.distributed(),
        }).toEqual({
            repeated: first.manifest,
            repeatedFiles: await first.reader.distributed(),
            isChanged: true,
            restored: first.manifest,
            restoredFiles: await first.reader.distributed(),
        });
    },
);
