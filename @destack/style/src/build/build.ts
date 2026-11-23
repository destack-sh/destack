import { unplugin } from "@stylexjs/unplugin";
import type { Environment } from "@destack/environment";
import type { Rollup, ViteDevServer } from "vite";
import { basename, join, relative } from "node:path";
import { type BuildExtension, LIGHT_DARK_FEATURE, type Plugin } from "@destack/package/build";
import { Digest, schema } from "@destack/schema";
import type { TransformCache } from "@destack/package/build";
import { BuildListCache, BuildReader } from "@destack/package/manifest";

/** The stylesheet module that carries the StyleX rules a document uses. */
const SHEET_ID = "virtual:stylex.css";

/** The stylesheet module's resolved identifier, prefixed as Vite marks virtual modules. */
const RESOLVED_SHEET_ID = `\0${SHEET_ID}`;

/** The milliseconds a sheet reload waits for further transforms, StyleX's polling interval. */
const SHEET_SETTLE = 150;

/** The file a build keeps its StyleX rules in, beside each output's modules. */
const RULES_FILE = "stylex.json";

/** One StyleX rule, as its babel plugin's `Rule` type has it. */
const Rule = schema.tuple([
    schema.string().min(1),
    schema.object({
        ltr: schema.string(),
        rtl: schema.string().nullable().exactOptional(),
        constKey: schema.string().exactOptional(),
        constVal: schema.union([schema.string(), schema.number()]).exactOptional(),
    }),
    schema.number(),
]);

/** The rules file a build keeps for the builds depending on it, by module path in its package. */
const RuleFile = schema.array(schema.tuple([schema.string().min(1), schema.array(Rule)]));

/** The packages whose imports StyleX compiles. */
const IMPORT_SOURCES = ["@stylexjs/stylex", "@destack/style"];

/**
 * The pattern of code that imports from StyleX or `@destack/style`.
 *
 * The bundler matches it natively before StyleX checks each module.
 */
const IMPORT_SOURCE_PATTERN = /@stylexjs\/stylex|@destack\/style/u;

/** The StyleX plugin with the rule collection it attaches without a type. */
interface StylexPlugin extends Plugin {
    /** Write every rule collected so far as one stylesheet. */
    readonly __stylexCollectCss: () => string;
    /** Read the rule store the plugin keeps on `globalThis` for the whole process. */
    readonly __stylexGetSharedStore: () => { readonly rulesById: Map<string, unknown> };
}

/**
 * The rules each dependency build keeps, read once per process by its directory.
 *
 * A dependency's directory is named by its build's digest, so its rules never change.
 */
const RULES = new Map<string, Promise<schema.Infer<typeof RuleFile>>>();

/** The directory a dependency's modules lie below in its build's tree. */
const NODE_MODULES = "/node_modules/";

/** The code of StyleX's plugin and of lightningcss, read once for the sheet's cache key. */
let SHEET_IDENTITY: Promise<readonly string[]> | undefined;

/** Compile StyleX styles in builds of packages that depend on `@destack/style`. */
export const styleExtension = {
    transform: ({ directory, dependencies, environment, cache }) => {
        // read the plugin StyleX declares without a type, with the base layer before its own
        const sheet: SheetTarget = { file: undefined };
        const plugin: unknown = unplugin.vite({
            importSources: IMPORT_SOURCES,
            useCSSLayers: { before: ["base"] },
            // keep light-dark() native, as the theme's own tokens need it and lowering it needs CSS-declared color schemes
            lightningcssOptions: { exclude: LIGHT_DARK_FEATURE },
            unstable_moduleResolution: { rootDir: directory, type: "commonJS" },
            cssInjectionTarget: (path) => path === sheet.file,
            devMode: "css-only",
        });
        if (!isStylexPlugin(plugin) || typeof plugin.transform !== "function") {
            throw new TypeError("stylex returned no vite plugin");
        }

        // drop the server hook serving the sheet a second way and polling it, as the sheet module does both, and the
        //  write hook, which collects the whole sheet again for a build without one
        const {
            configureServer: _polling,
            writeBundle: _written,
            generateBundle: _generated,
            ...compiler
        } = plugin;

        // let the bundler skip each module without an import source before StyleX reads it
        const filtered: Plugin = {
            ...compiler,
            transform: { filter: { code: IMPORT_SOURCE_PATTERN }, handler: plugin.transform },
            // inject the rules only into a sheet, as a build without one keeps them for its dependents
            async generateBundle(_options, bundle) {
                // leave a build without a sheet
                if (sheet.file === undefined) {
                    return;
                }

                // append the sheet its rules make, read from the cache by their digest
                const css = await cachedSheet(plugin, cache, environment);
                injectSheet(this, bundle, sheet.file, css);
            },
        };

        // collect the dependencies' rules into the sheet beside the compiler
        const built = Object.values(dependencies);

        return [filtered, sheetPlugin(plugin, sheet, directory, built, environment)];
    },
} satisfies BuildExtension;

/** The stylesheet a build appends the StyleX rules to, set before StyleX writes them. */
interface SheetTarget {
    /** The stylesheet of the chunk that imports the sheet module. */
    file: string | undefined;
}

/** Collect the dependencies' StyleX rules into the stylesheet, and keep the build's own. */
function sheetPlugin(
    stylex: StylexPlugin,
    sheet: SheetTarget,
    directory: string,
    dependencies: readonly string[],
    environment: Environment<"files">,
): Plugin {
    // the server, the class names of the sheet last served, and the queued reload
    let server: ViteDevServer | undefined;
    let served = new Set<string>();
    let isReloadQueued = false;

    return {
        name: "destack:style-sheet",
        configureServer: (devServer) => {
            server = devServer;
        },
        buildStart: async () => {
            // forget the rules earlier builds of this process collected, as StyleX keeps them for the whole process
            const rules = stylex.__stylexGetSharedStore().rulesById;
            rules.clear();

            // add the rules each dependency's build keeps under its modules, ordered with this build's own
            for (const dependency of dependencies) {
                let read = RULES.get(dependency);
                if (read === undefined) {
                    read = readRules(dependency, environment);
                    RULES.set(dependency, read);
                }
                for (const [module, rule] of await read) {
                    rules.set(join(dependency, module), rule);
                }
            }
        },
        resolveId: (id) => (withoutQuery(id) === SHEET_ID ? RESOLVED_SHEET_ID : undefined),
        load: (id) => {
            // serve the rules in development, and nothing in builds, where StyleX appends them to this module's stylesheet
            if (withoutQuery(id) !== RESOLVED_SHEET_ID) {
                return undefined;
            }
            served = new Set(
                [...stylex.__stylexGetSharedStore().rulesById.values()].flatMap((rules) =>
                    schema
                        .array(Rule)
                        .parse(rules)
                        .map(([name]) => name),
                ),
            );

            return server === undefined ? "" : stylex.__stylexCollectCss();
        },
        generateBundle: {
            order: "pre",
            handler(_options, bundle) {
                // aim StyleX at the stylesheet of the chunk importing the sheet module
                const chunks = Object.values(bundle).filter((output) => output.type === "chunk");
                const holder = chunks.find((chunk) => chunk.moduleIds.includes(RESOLVED_SHEET_ID));
                sheet.file = holder?.viteMetadata?.importedCss.values().next().value;

                // keep the build's own rules for the builds depending on it, sorted by module path in the package
                const rules = [...stylex.__stylexGetSharedStore().rulesById]
                    .map(([module, rule]) => [relative(directory, module), rule] as const)
                    .filter(([module]) => !module.startsWith(".."))
                    .toSorted(([left], [right]) => (left < right ? -1 : Number(left > right)));
                if (rules.length > 0) {
                    this.emitFile({
                        type: "asset",
                        fileName: RULES_FILE,
                        source: JSON.stringify(rules),
                    });
                }
            },
        },
        transform: (_code, id) => {
            // leave a transform that adds no rule the served sheet lacks, or one a queued reload covers
            const running = server;
            const rules = stylex.__stylexGetSharedStore().rulesById.get(id);
            if (running === undefined || isReloadQueued || rules === undefined) {
                return undefined;
            }
            const names = schema
                .array(Rule)
                .parse(rules)
                .map(([name]) => name);
            if (names.every((name) => served.has(name))) {
                return undefined;
            }

            // send the browser the sheet with the new rules once the burst of transforms settles
            isReloadQueued = true;
            void environment.clock.sleep(SHEET_SETTLE).then(() => {
                // reload the sheet module the browser imported, if it has
                isReloadQueued = false;
                const client = running.environments.client;
                const module = client.moduleGraph.getModuleById(RESOLVED_SHEET_ID);
                if (module !== undefined) {
                    void client.reloadModule(module);
                }
            });

            return undefined;
        },
    };
}

/** Read the StyleX rules a dependency's build keeps, in the rules files its file list has. */
async function readRules(
    directory: string,
    environment: Pick<Environment<"files">, "files" | "crypto">,
): Promise<schema.Infer<typeof RuleFile>> {
    // read the build's file list, then each output's rules file
    const build = await BuildReader.openDirectory(
        directory,
        environment.files,
        environment.crypto,
        new BuildListCache(),
    );
    const listed = (await build.files()).filter((file) => basename(file.path) === RULES_FILE);
    const decoder = new TextDecoder();
    const rules = await Promise.all(
        listed.map(async (file) =>
            RuleFile.parse(JSON.parse(decoder.decode(await build.load(file.path)))),
        ),
    );

    return rules.flat();
}

/** Read the sheet a build's rules make from the cache by their digest, else collect and keep it. */
async function cachedSheet(
    plugin: StylexPlugin,
    cache: TransformCache | undefined,
    environment: Pick<Environment<"files">, "files" | "crypto">,
): Promise<string> {
    // key the sheet by StyleX's and lightningcss's code and the rules, in module order
    SHEET_IDENTITY ??= Promise.all(
        [import.meta.resolve("@stylexjs/unplugin"), import.meta.resolve("lightningcss")].map(
            (url) => environment.files.readTextFile(new URL(url).pathname),
        ),
    );
    const rules = [...plugin.__stylexGetSharedStore().rulesById]
        .map(([id, rule]) => [packagePath(id), rule] as const)
        .toSorted(([left], [right]) => (left < right ? -1 : Number(left > right)));
    const key = await Digest.of(
        [...(await SHEET_IDENTITY), JSON.stringify(rules)].join("\0"),
        environment.crypto,
    );

    // take the kept sheet, else collect it and keep it for later builds
    const kept = await cache?.read(key);
    if (kept !== undefined) {
        return new TextDecoder().decode(kept);
    }
    const css = plugin.__stylexCollectCss();
    await cache?.write(key, new TextEncoder().encode(css));

    return css;
}

/** Append CSS to a stylesheet under a name its content hashes to, renamed where it is named. */
function injectSheet(
    context: Pick<Rollup.PluginContext, "emitFile" | "getFileName">,
    bundle: Rollup.OutputBundle,
    file: string,
    css: string,
): void {
    // emit the stylesheet with the CSS appended, under the name its content hashes to
    const asset = bundle[file];
    if (asset?.type !== "asset") {
        throw new TypeError(`stylesheet '${file}' is not an asset of the bundle`);
    }
    const current =
        typeof asset.source === "string" ? asset.source : new TextDecoder().decode(asset.source);
    const source = current === "" ? css : `${current}\n${css}`;
    const renamed = context.getFileName(
        context.emitFile({ type: "asset", name: asset.names[0] ?? basename(file), source }),
    );
    if (renamed === file) {
        asset.source = source;

        return;
    }

    // find the chunks importing it, and those reaching them by static imports, whose loads preload it
    const chunks = Object.values(bundle).filter((output) => output.type === "chunk");
    const holders = new Set(
        chunks
            .filter((chunk) => chunk.viteMetadata?.importedCss.has(file) === true)
            .map((chunk) => chunk.fileName),
    );
    const reaching = new Set(holders);
    for (let size = 0; size !== reaching.size;) {
        size = reaching.size;
        for (const chunk of chunks) {
            if (chunk.imports.some((imported) => reaching.has(imported))) {
                reaching.add(chunk.fileName);
            }
        }
    }

    // rename it in the chunks importing it and in those preloading them, reading no other chunk's code
    for (const chunk of chunks) {
        const isNaming =
            holders.has(chunk.fileName) ||
            chunk.dynamicImports.some((imported) => reaching.has(imported));
        if (isNaming) {
            chunk.code = chunk.code.replaceAll(file, renamed);
            chunk.viteMetadata?.importedCss.delete(file);
            if (holders.has(chunk.fileName)) {
                chunk.viteMetadata?.importedCss.add(renamed);
            }
        }
    }

    // rename it in the text assets, such as a manifest, and drop the old one
    for (const output of Object.values(bundle)) {
        if (output.type === "asset" && typeof output.source === "string") {
            output.source = output.source.replaceAll(file, renamed);
        }
    }
    Reflect.deleteProperty(bundle, file);
}

/** Read a module's path within its package, a dependency's below its build's digest-named tree. */
function packagePath(id: string): string {
    const below = id.lastIndexOf(NODE_MODULES);

    return below === -1 ? id : id.slice(below + NODE_MODULES.length);
}

/** Remove a module identifier's query, such as the `?direct` of a stylesheet's text. */
function withoutQuery(id: string): string {
    return id.replace(/\?.*$/su, "");
}

/** Check that a value is the StyleX Vite plugin with its rule collection. */
function isStylexPlugin(value: unknown): value is StylexPlugin {
    return (
        typeof value === "object" &&
        value !== null &&
        "name" in value &&
        typeof value.name === "string" &&
        "__stylexCollectCss" in value &&
        typeof value.__stylexCollectCss === "function" &&
        "__stylexGetSharedStore" in value &&
        typeof value.__stylexGetSharedStore === "function"
    );
}
