import { unplugin } from "@stylexjs/unplugin";
import type { Environment } from "@destack/environment";
import { Features } from "lightningcss";
import type { ViteDevServer } from "vite";
import { basename, join, relative } from "node:path";
import type { BuildExtension, Plugin } from "@destack/package/build";
import { schema } from "@destack/schema";
import { BuildReader } from "@destack/package/manifest";

/** The stylesheet module that carries the StyleX rules a document uses. */
const SHEET_ID = "virtual:stylex.css";

/** The stylesheet module's resolved identifier, prefixed as Vite marks virtual modules. */
const RESOLVED_SHEET_ID = `\0${SHEET_ID}`;

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
    readonly __stylexGetSharedStore: () => {
        readonly version: number;
        readonly rulesById: Map<string, unknown>;
    };
}

/** Compile StyleX styles in builds of packages that depend on `@destack/style`. */
export const styleExtension = {
    transform: ({ directory, dependencies, environment }) => {
        // read the plugin StyleX declares without a type, with the base layer before its own
        const sheet: SheetTarget = { file: undefined };
        const plugin: unknown = unplugin.vite({
            importSources: IMPORT_SOURCES,
            useCSSLayers: { before: ["base"] },
            // keep light-dark() native, as the theme's own tokens need it and lowering it needs CSS-declared color schemes
            lightningcssOptions: { exclude: Features.LightDark },
            unstable_moduleResolution: { rootDir: directory, type: "commonJS" },
            cssInjectionTarget: (path) => path === sheet.file,
            devMode: "css-only",
        });
        if (!isStylexPlugin(plugin) || typeof plugin.transform !== "function") {
            throw new TypeError("stylex returned no vite plugin");
        }

        // drop the server hook serving the sheet a second way and polling it, as the sheet module does both
        const { configureServer: _polling, generateBundle, ...compiler } = plugin;

        // let the bundler skip each module without an import source before StyleX reads it
        const filtered: Plugin = {
            ...compiler,
            transform: { filter: { code: IMPORT_SOURCE_PATTERN }, handler: plugin.transform },
            // inject the rules only into a sheet, as a build without one keeps them for its dependents
            generateBundle(...args) {
                const handler =
                    typeof generateBundle === "function" ? generateBundle : generateBundle?.handler;

                return sheet.file === undefined ? undefined : handler?.apply(this, args);
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
    // hold the development server, the rule versions the sheet last carried and was last reloaded for, and a queued reload
    let server: ViteDevServer | undefined;
    let version = 0;
    let reloaded = 0;
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
                for (const [module, rule] of await readRules(dependency, environment)) {
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
            version = stylex.__stylexGetSharedStore().version;

            return server === undefined ? "" : stylex.__stylexCollectCss();
        },
        generateBundle: {
            order: "pre",
            handler(_options, bundle) {
                // aim StyleX at the stylesheet of the chunk importing the sheet module
                const chunks = Object.values(bundle).filter((output) => output.type === "chunk");
                const holder = chunks.find((chunk) => chunk.moduleIds.includes(RESOLVED_SHEET_ID));
                sheet.file = holder?.viteMetadata?.importedCss.values().next().value;

                // keep the build's own rules for the builds depending on it, by module path in the package
                const rules = [...stylex.__stylexGetSharedStore().rulesById]
                    .filter(([module]) => !relative(directory, module).startsWith(".."))
                    .map(([module, rule]) => [relative(directory, module), rule]);
                if (rules.length > 0) {
                    this.emitFile({
                        type: "asset",
                        fileName: RULES_FILE,
                        source: JSON.stringify(rules),
                    });
                }
            },
        },
        transform: () => {
            // reload the sheet once per task after transforms in any environment add rules
            const running = server;
            if (running === undefined || isReloadQueued) {
                return undefined;
            }
            const current = stylex.__stylexGetSharedStore().version;
            if (current === version || current === reloaded) {
                return undefined;
            }
            isReloadQueued = true;
            reloaded = current;
            void environment.clock.sleep(0).then(() => {
                // send the browser the sheet with the new rules
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
    const build = await BuildReader.openDirectory(directory, environment.files, environment.crypto);
    const listed = (await build.files()).filter((file) => basename(file.path) === RULES_FILE);
    const decoder = new TextDecoder();
    const rules = await Promise.all(
        listed.map(async (file) =>
            RuleFile.parse(JSON.parse(decoder.decode(await build.load(file.path)))),
        ),
    );

    return rules.flat();
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
