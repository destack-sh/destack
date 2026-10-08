import { unplugin } from "@stylexjs/unplugin";
import type { ViteDevServer } from "vite";
import type { BuildExtension, Plugin } from "@destack/package/build";

/** The stylesheet module that carries the StyleX rules a document uses. */
const SHEET_ID = "virtual:stylex.css";

/** The stylesheet module's resolved identifier, prefixed as Vite marks virtual modules. */
const RESOLVED_SHEET_ID = `\0${SHEET_ID}`;

/** The StyleX plugin with the rule collection it attaches without a type. */
interface StylexPlugin extends Plugin {
    /** Write every rule collected so far as one stylesheet. */
    readonly __stylexCollectCss: () => string;
    /** Read the rule store, whose version grows with each transform that changes rules. */
    readonly __stylexGetSharedStore: () => { readonly version: number };
}

/** Compile StyleX styles in builds of packages that depend on `@destack/style`. */
export const styleExtension = {
    transform: ({ directory }) => {
        // read the plugin StyleX declares without a type, with the base layer before its own
        const sheet: SheetTarget = { file: undefined };
        const plugin: unknown = unplugin.vite({
            importSources: ["@stylexjs/stylex", "@destack/style"],
            useCSSLayers: { before: ["base"] },
            unstable_moduleResolution: { rootDir: directory, type: "commonJS" },
            cssInjectionTarget: (path) => path === sheet.file,
            // NOTE #Performance: the plugin still polls its rule version every 150ms in this mode
            devMode: "css-only",
        });
        if (!isStylexPlugin(plugin)) {
            throw new TypeError("stylex returned no vite plugin");
        }

        return [plugin, sheetPlugin(plugin, sheet)];
    },
} satisfies BuildExtension;

/** The stylesheet a build appends the StyleX rules to, set before StyleX writes them. */
interface SheetTarget {
    /** The stylesheet the entry chunk loads, absent before the bundle is written. */
    file: string | undefined;
}

/** Serve the collected StyleX rules as a stylesheet module in development, and aim builds at the entry's stylesheet. */
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
        resolveId: (id) => (withoutQuery(id) === SHEET_ID ? RESOLVED_SHEET_ID : undefined),
        load: (id) => {
            // serve the rules in development, and nothing in builds, where StyleX appends the rules to the entry's stylesheet
            if (withoutQuery(id) !== RESOLVED_SHEET_ID) {
                return undefined;
            }
            version = stylex.__stylexGetSharedStore().version;

            return server === undefined ? "" : stylex.__stylexCollectCss();
        },
        generateBundle: {
            order: "pre",
            handler(_options, bundle) {
                // aim StyleX at the stylesheet the entry chunk loads, which every page of the bundle loads
                const chunks = Object.values(bundle).filter((output) => output.type === "chunk");
                const stylesheets = chunks.flatMap((chunk) => [
                    ...(chunk.viteMetadata?.importedCss ?? []),
                ]);
                sheet.file = chunks
                    .filter((chunk) => chunk.isEntry)
                    .flatMap((chunk) => [...(chunk.viteMetadata?.importedCss ?? [])])[0];

                // refuse rules in a bundle whose stylesheets all belong to lazy chunks
                if (
                    stylesheets.length > 0 &&
                    sheet.file === undefined &&
                    stylex.__stylexCollectCss() !== ""
                ) {
                    this.error("styles need a stylesheet the entry loads");
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
