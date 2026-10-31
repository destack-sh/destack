import { join, sep } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import type {
    BuildExtension,
    Compilation,
    CompiledOutput,
    DeclarationModule,
    Plugin,
    PrerenderContext,
} from "@destack/package/build";
import { BROWSER_SANDBOX, type DeclarationDescription, Sandbox } from "@destack/package";
import {
    CommandDescription,
    type OutputContent,
    ViewDescription,
    ViewRender,
} from "@destack/package/manifest";
import { present } from "@destack/schema";
import { destackTheme } from "@destack/theme/declare";
import { fontPlugin } from "./font.ts";
import { imagePlugin } from "./image.ts";
import { islandPlugin } from "./island.ts";
import { prerender } from "./prerender/index.ts";
import { pagesPlugin } from "./pages.ts";
import { VIEW_PACKAGE, viewPlugins } from "./plugin.ts";
import { ViewError } from "../error/error.ts";

/** The prefix of the generated modules rendering each view. */
export const PREFIX = "virtual:@destack/view/view/";

/** The path below a static view's pages its server-rendered assets publish at. */
const ASSET_PATH = ".destack/asset/";

/** The directory of the assets an output emits, as the build names them. */
const ASSET_DIRECTORY = "asset/";

/** The directory of a package's files published as they are beside its pages, as Vite's. */
const PUBLIC_DIRECTORY = "public";

/** The package's own island modules an islands page loads, as Vite globs them. */
const ISLAND_MODULES = ["/**/islands/*.{tsx,jsx}", "!**/node_modules/**"];

/** The stylesheets each view's page links: the base styles, then the build's StyleX rules. */
const STYLESHEETS = [`${VIEW_PACKAGE}/styles.css`, "virtual:stylex.css"] as const;

/** A view's render as its declaration describes it. */
type DeclaredRender = ReturnType<typeof ViewRender.declaration.parse>;

/** What locating a package's views reads: the package, its declarations and their modules. */
export type ViewPackage = Pick<Compilation, "package" | "declarations" | "locate">;

/** A static view's render as its declaration describes it. */
export type StaticRender = Extract<DeclaredRender, { readonly mode: "static" }>;

/** Compile Solid components wherever modules compile, and each view's entries and pages. */
export const viewExtension: BuildExtension = {
    transform(context) {
        // remove static views' pages modules from browser modules, and wrap the package's islands
        return [
            ...(context.runtime === "browser" ? [pagesPlugin()] : []),
            islandPlugin(context.directory),
            ...viewPlugins(context.server),
            imagePlugin(context.environment, context.cache),
            fontPlugin(context.environment.files),
        ];
    },
    compile(compilation) {
        // emit each view's browser entry, and each static view's server entry on Bun
        const views = locate(compilation).filter(
            (view) =>
                compilation.runtime === "browser" ||
                (compilation.runtime === "bun" && view.render.mode === "static"),
        );
        for (const view of views) {
            compilation.entry(`./view/${view.name}`, `${PREFIX}${view.name}`);
        }

        // address the server's assets where its pages publish them
        const isRendering = compilation.runtime === "bun" && views.length > 0;

        return [entryPlugin(views, compilation), ...(isRendering ? [assetPlugin()] : [])];
    },
    describe(compilation, compiled) {
        // describe each view of a browser output by its entry, stylesheets, browser abilities and presented types
        if (compilation.runtime !== "browser") {
            return {};
        }
        const declared = compilation.sandbox;
        const sandbox = Sandbox.select(
            declared,
            BROWSER_SANDBOX.filter((name) => declared[name] !== undefined),
        );
        const views: Record<string, ViewDescription> = {};
        const located = locate(compilation);
        requireOneTheme(compilation, located.length);
        for (const view of located) {
            const entrypoint = `./view/${view.name}`;
            const emitted = compiled.exports[entrypoint];
            if (emitted === undefined) {
                throw new TypeError(`missing view entrypoint: ${view.name}`);
            }
            views[view.name] = {
                entrypoint: emitted,
                stylesheets: pageStylesheets(compilation, compiled, view),
                sandbox,
                presents: view.presents,
                categories: view.categories,
                packages: view.packages,
                render: { mode: view.render.mode === "static" ? "static" : "client" },
            };
        }

        return { views };
    },
    prerender: {
        isRendering: (declarations) =>
            declarations.some((declaration) => staticRender(declaration) !== undefined),
        render: prerenderViews,
    },
};

/** Render each static view's pages and endpoints, beside the package's public files. */
async function prerenderViews(
    context: PrerenderContext,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // render each static view's pages and endpoints, beside the package's public files
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    for (const declaration of context.declarations) {
        // select static views
        const render = staticRender(declaration);
        if (render === undefined) {
            continue;
        }

        // find the browser output describing the view and the Bun module rendering it
        const { name } = declaration;
        const { output, view } = describedView(context.outputs, name);
        const server = Object.values(context.outputs).find(
            (each) => each.runtime === "bun" && each.exports[`./view/${name}`] !== undefined,
        );
        const module = server?.exports[`./view/${name}`]?.path;
        if (module === undefined) {
            throw new TypeError(`the build compiled no Bun output rendering view ${name}`);
        }

        // render its routes with the view's entry and stylesheets
        const rendered = await prerender(
            join(context.build, module),
            render.origin,
            {
                script: `/${view.entrypoint}`,
                stylesheets: view.stylesheets.map((path) => `/${path}`),
            },
            context.environment,
        );
        const directory = ViewDescription.pages(output, name);
        for (const [path, bytes] of rendered) {
            publish(files, `${directory}/${path}`, bytes);
        }

        // publish the assets its pages load, and the package's public files, beside its pages
        const { files: system } = context.environment;
        const assets = join(
            context.build,
            present(server, "the view's bun output").directory,
            ASSET_DIRECTORY,
        );
        for (const [path, bytes] of await filesBelow(assets, system)) {
            publish(files, `${directory}/${ASSET_PATH}${path}`, bytes);
        }
        for (const [path, bytes] of await filesBelow(
            join(context.directory, PUBLIC_DIRECTORY),
            system,
        )) {
            publish(files, `${directory}/${path}`, bytes);
        }
    }

    return files;
}

/** Publish a file beside a static view's pages, refusing a path another file took. */
function publish(
    files: Map<string, Uint8Array<ArrayBuffer>>,
    path: string,
    bytes: Uint8Array<ArrayBuffer>,
): void {
    if (files.has(path)) {
        throw ViewError.of("PAGE_DUPLICATE", `two files publish at ${path}`);
    }
    files.set(path, bytes);
}

/** Address each asset a server output emits below its pages' asset path. */
function assetPlugin(): Plugin {
    return {
        name: `${VIEW_PACKAGE}/asset`,
        resolveFileUrl: ({ fileName }) =>
            fileName.startsWith(ASSET_DIRECTORY)
                ? JSON.stringify(`/${ASSET_PATH}${fileName.slice(ASSET_DIRECTORY.length)}`)
                : null,
    };
}

/** Read the files below a directory by path, none when it is absent. */
async function filesBelow(
    root: string,
    files: FileSystem,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // list the directory's files
    const entries = await files
        .readDirectory(root, { recursive: true })
        .catch(FileSystemError.absent);
    const read = new Map<string, Uint8Array<ArrayBuffer>>();
    for (const entry of (entries ?? []).filter((each) => each.kind === "file")) {
        read.set(entry.name.split(sep).join("/"), await files.readFile(join(root, entry.name)));
    }

    return read;
}

/** List a view's stylesheets: its entry's, then those of the module declaring it. */
function pageStylesheets(
    compilation: Compilation,
    compiled: CompiledOutput,
    view: LocatedView,
): string[] {
    const declaring = Object.entries(compilation.exports).find(
        ([, source]) => source === view.file,
    );
    const own = declaring === undefined ? [] : compiled.stylesheets(declaring[0]);

    return [...new Set([...compiled.stylesheets(`./view/${view.name}`), ...own])];
}

/** Refuse a package with views that declares more than one theme. */
function requireOneTheme(compilation: Compilation, views: number): void {
    const themes = compilation.declarations.filter(
        (declaration) =>
            declaration.kind === "theme" &&
            declaration.symbol.package.id === compilation.package.id,
    );
    if (views > 0 && themes.length > 1) {
        throw ViewError.of(
            "MULTIPLE_THEMES",
            `a package with views declares at most one theme: ${themes.map((theme) => theme.name).join(", ")}`,
        );
    }
}

/** A view the package declares, with the module exporting it. */
export interface LocatedView extends DeclarationModule {
    /** The view's name. */
    readonly name: string;
    /** The object types the view presents. */
    readonly presents: ViewDescription["presents"];
    /** The roles clients open the view in. */
    readonly categories: ViewDescription["categories"];
    /** The packages the view calls. */
    readonly packages: ViewDescription["packages"];
    /** How the view renders. */
    readonly render: DeclaredRender;
}

/** Locate the views the package declares, refusing a server view. */
export function locate(compilation: ViewPackage): LocatedView[] {
    const views: LocatedView[] = [];
    for (const declaration of compilation.declarations) {
        // select views this package declares
        if (!isView(declaration) || declaration.symbol.package.id !== compilation.package.id) {
            continue;
        }

        // refuse a name taken twice
        if (views.some((view) => view.name === declaration.name)) {
            throw ViewError.of("VIEW_DUPLICATE", `duplicate view: ${declaration.name}`);
        }

        // refuse a server view
        // TODO #Incomplete: compile server views and render them in the workload as the visitor
        const { description } = declaration;
        const render = ViewRender.declaration.parse(description["render"]);
        if (render.mode === "server") {
            throw ViewError.of(
                "RENDER_UNSUPPORTED",
                `view ${declaration.name} renders on the server, which builds do not compile yet`,
            );
        }

        // keep the module exporting the view, the types it presents, its roles, packages and render
        views.push({
            name: declaration.name,
            ...compilation.locate(declaration),
            presents: ViewDescription.shape.presents.parse(description["presents"]),
            categories: ViewDescription.shape.categories.parse(description["categories"]),
            packages: ViewDescription.shape.packages.parse(description["packages"]),
            render,
        });
    }

    return views;
}

/** Report whether a declaration is a view. */
function isView(declaration: DeclarationDescription): boolean {
    return declaration.kind === "view" && declaration.package.name === VIEW_PACKAGE;
}

/** Read a declaration's static render, absent for any other declaration. */
function staticRender(declaration: DeclarationDescription): StaticRender | undefined {
    // select views
    if (!isView(declaration)) {
        return undefined;
    }

    // select static views
    const render = ViewRender.declaration.parse(declaration.description["render"]);

    return render.mode === "static" ? render : undefined;
}

/** Find the browser output describing a view, with its description. */
function describedView(
    outputs: Readonly<Record<string, OutputContent>>,
    name: string,
): { readonly output: OutputContent; readonly view: OutputContent["views"][string] } {
    for (const output of Object.values(outputs)) {
        const view = output.views[name];
        if (view !== undefined) {
            return { output, view };
        }
    }

    throw ViewError.of(
        "RUNTIME_REQUIRED",
        `static view ${name} hydrates in the browser: add browser to the package's runtimes`,
    );
}

/** Load each view's generated entry with the package's theme and commands. */
export function entryPlugin(views: readonly LocatedView[], compilation: ViewPackage): Plugin {
    // read the theme and the commands with their key combinations the package declares
    const own = compilation.declarations.filter(
        (declaration) => declaration.symbol.package.id === compilation.package.id,
    );
    const theme = own.find(
        (declaration) =>
            declaration.kind === "theme" && declaration.package.id === destackTheme.package.id,
    )?.description;
    const commands = own
        .filter(
            (declaration) =>
                declaration.kind === "command" && declaration.package.name === VIEW_PACKAGE,
        )
        .map((declaration) => {
            const { keybinding } = CommandDescription.parse(declaration.description);

            return {
                packageId: compilation.package.id,
                name: declaration.name,
                ...(keybinding === undefined ? {} : { keybinding }),
            };
        });
    const declared = JSON.stringify({ commands, ...(theme === undefined ? {} : { theme }) });
    const generated = new RegExp(`^\\0${RegExp.escape(PREFIX)}`, "u");

    return {
        name: VIEW_PACKAGE,
        resolveId: {
            filter: {
                id: new RegExp(
                    `^(?:${RegExp.escape(PREFIX)}|${RegExp.escape(STYLESHEETS[0])}$)`,
                    "u",
                ),
            },
            async handler(source, origin) {
                // resolve a generated module, and its stylesheet from the module declaring its view
                const view = views.find((entry) => `\0${PREFIX}${entry.name}` === origin);
                if (source.startsWith(PREFIX)) {
                    return `\0${source}`;
                } else if (view !== undefined) {
                    return await this.resolve(source, view.file, { skipSelf: true });
                }

                return null;
            },
        },
        load: {
            filter: { id: generated },
            handler(id, options) {
                // write the module of the view the id has, for the server or the browser
                const view = views.find((entry) => `\0${PREFIX}${entry.name}` === id);
                const isServer = options?.ssr === true;

                return view && entryModule(view, isServer, compilation, declared);
            },
        },
    };
}

/** Write a view's entry: rendering its pages on the server, or mounting or hydrating it. */
function entryModule(
    view: LocatedView,
    isServer: boolean,
    compilation: ViewPackage,
    declared: string,
): string {
    const imported = `import { ${view.export} as view } from ${JSON.stringify(view.file)};`;
    const stylesheets = STYLESHEETS.map((path) => `import ${JSON.stringify(path)};`);

    // render a static view's pages on the server, linking the page's assets each request names
    if (isServer) {
        return [
            `import { BrowserEnvironment, Environment, StaticPage } from "${VIEW_PACKAGE}/static";`,
            imported,
            ``,
            `const page = await StaticPage.open(view, ${declared}, BrowserEnvironment.server(Environment.system));`,
            ``,
            `export function routes() {`,
            `    return page.routes();`,
            `}`,
            ``,
            `export function respond(request, assets) {`,
            `    return page.respond(request, assets);`,
            `}`,
            ``,
        ].join("\n");
    }
    // mount a client view
    else if (view.render.mode !== "static") {
        return [
            ...stylesheets,
            `import { BrowserEnvironment, mount } from "${VIEW_PACKAGE}/browser";`,
            imported,
            ``,
            `await mount(view, ${declared}, BrowserEnvironment.system);`,
            ``,
        ].join("\n");
    }
    // hydrate a static view's page whole
    else if (view.render.hydrate === "page") {
        return [
            ...stylesheets,
            `import { BrowserEnvironment, hydrateStatic } from "${VIEW_PACKAGE}/static/browser";`,
            imported,
            ``,
            `await hydrateStatic(view, ${declared}, BrowserEnvironment.system);`,
            ``,
        ].join("\n");
    }
    // hydrate only the components of the package's islands directories, as Fresh does, loading no view code
    else {
        const page = JSON.stringify({ name: view.name, package: compilation.package });

        return [
            ...stylesheets,
            `import { BrowserEnvironment, hydrateIslands } from "${VIEW_PACKAGE}/static/browser";`,
            ``,
            `const islands = import.meta.glob(${JSON.stringify(ISLAND_MODULES)}, { import: "default" });`,
            `await hydrateIslands(${page}, islands, BrowserEnvironment.system);`,
            ``,
        ].join("\n");
    }
}
