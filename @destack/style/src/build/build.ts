import { unplugin } from "@stylexjs/unplugin";
import { Features } from "lightningcss";
import type { ViteDevServer } from "vite";
import type { BuildExtension, Plugin } from "@destack/package/build";

/** The stylesheet module that carries the StyleX rules a document uses. */
const SHEET_ID = "virtual:stylex.css";

/** The stylesheet module's resolved identifier, prefixed as Vite marks virtual modules. */
const RESOLVED_SHEET_ID = `\0${SHEET_ID}`;

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
    transform: ({ directory }) => {
        // read the plugin StyleX declares without a type, with the base layer before its own
        const sheet: SheetTarget = { file: undefined };
        const plugin: unknown = unplugin.vite({
            importSources: IMPORT_SOURCES,
            useCSSLayers: { before: ["base"] },
            // keep light-dark() native, as the theme's own tokens need it and lowering it needs CSS-declared color schemes
            lightningcssOptions: { exclude: Features.LightDark },
            unstable_moduleResolution: { rootDir: directory, type: "commonJS" },
            cssInjectionTarget: (path) => path === sheet.file,
            // NOTE #Performance: the plugin still polls its rule version every 150ms in this mode
            devMode: "css-only",
        });
        if (!isStylexPlugin(plugin) || typeof plugin.transform !== "function") {
            throw new TypeError("stylex returned no vite plugin");
        }

        // let the bundler skip each module naming no import source before StyleX reads it
        const filtered: Plugin = {
            ...plugin,
            transform: { filter: { code: IMPORT_SOURCE_PATTERN }, handler: plugin.transform },
        };

        return [filtered, sheetPlugin(plugin, sheet)];
    },
} satisfies BuildExtension;

/** The stylesheet a build appends the StyleX rules to, set before StyleX writes them. */
interface SheetTarget {
    /** The stylesheet of the chunk that imports the sheet module. */
    file: string | undefined;
}

/** Serve the collected StyleX rules as a stylesheet module in development. */
function sheetPlugin(stylex: StylexPlugin, sheet: SheetTarget): Plugin {
    // hold the development server, the rule version the sheet last carried and a queued reload
    let server: ViteDevServer | undefined;
    let version = 0;
    let isReloadQueued = false;

    return {
        name: "destack:style-sheet",
        configureServer: (devServer) => {
            server = devServer;
        },
        buildStart: () => {
            // forget the rules earlier builds of this process collected, as StyleX keeps them for the whole process
            stylex.__stylexGetSharedStore().rulesById.clear();
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

                // refuse rules in a bundle with stylesheets that never imports the sheet module
                const isStyled = chunks.some(
                    (chunk) => (chunk.viteMetadata?.importedCss.size ?? 0) > 0,
                );
                if (isStyled && sheet.file === undefined && stylex.__stylexCollectCss() !== "") {
                    this.error(`styles need a module that imports ${SHEET_ID}`);
                }
            },
        },
        transform: () => {
            // reload the sheet once per task after transforms in any environment add rules
            const running = server;
            if (running === undefined || isReloadQueued) {
                return undefined;
            }
            if (stylex.__stylexGetSharedStore().version === version) {
                return undefined;
            }
            isReloadQueued = true;
            setTimeout(() => {
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
