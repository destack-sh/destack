import * as stylex from "@stylexswc/rs-compiler";
import { transform } from "lightningcss";
import type { Environment } from "@destack/environment";
import { basename, join, relative } from "node:path";
import { type BuildExtension, type Plugin, runtimeCss } from "@destack/package/build";
import { Digest, schema } from "@destack/schema";
import type { TransformCache } from "@destack/package/build";
import { BuildListCache, BuildReader } from "@destack/package/manifest";

/** The stylesheet module that carries the StyleX rules a document uses. */
const SHEET_ID = "virtual:stylex.css";

/** The stylesheet module's resolved identifier, prefixed as Vite marks virtual modules. */
const RESOLVED_SHEET_ID = `\0${SHEET_ID}`;

/** The name of the stylesheet each build writes its StyleX rules into. */
const SHEET_FILE = "stylex.css";

/** The file a build keeps its StyleX rules in, beside each output's modules. */
const RULES_FILE = "stylex.json";

/** One StyleX rule: its class name, its left-to-right and right-to-left text, and its priority. */
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
/** One StyleX rule. */
type Rule = schema.Infer<typeof Rule>;

/** The rules file a build keeps for the builds depending on it, by module path in its package. */
const RuleFile = schema.array(schema.tuple([schema.string().min(1), schema.array(Rule)]));

/** The package of the styles. */
const STYLE_PACKAGE = "@destack/style";

/** The packages whose imports StyleX compiles. */
const IMPORT_SOURCES = ["@stylexjs/stylex", STYLE_PACKAGE];

/**
 * The pattern of code that imports from StyleX or `@destack/style`.
 *
 * The bundler matches it natively before StyleX reads each module.
 */
const IMPORT_SOURCE_PATTERN = /@stylexjs\/stylex|@destack\/style/u;

/** The modules StyleX compiles: JavaScript and TypeScript, with or without JSX. */
const MODULE_PATTERN = /\.[cm]?[jt]sx?(?:\?|$)/u;

/** The layers StyleX's rules come after: the base layer of the preflight and the themes. */
const LAYERS = { before: ["base"] };

/**
 * The rules each dependency build keeps, read once per process by its directory.
 *
 * A dependency's directory is named by its build's digest, so its rules never change.
 */
const RULES = new Map<string, Promise<schema.Infer<typeof RuleFile>>>();

/** The directory a dependency's modules lie below in its build's tree. */
const NODE_MODULES = "/node_modules/";

/** The code of StyleX's compiler and of Lightning CSS, read once for the sheet's cache key. */
let SHEET_IDENTITY: Promise<readonly string[]> | undefined;

/** Compile StyleX styles in builds of packages that depend on `@destack/style`. */
export const styleExtension = {
    transform: ({ directory, dependencies, environment, cache }) => [
        stylePlugin(directory, Object.values(dependencies), environment, cache),
    ],
} satisfies BuildExtension;

/** Compile each module's styles, keep the build's rules, and append every rule to the sheet. */
function stylePlugin(
    directory: string,
    dependencies: readonly string[],
    environment: Environment<"files">,
    cache: TransformCache | undefined,
): Plugin {
    // compile as a production build resolves, each module's rules by its identifier
    const options: stylex.StyleXOptions = {
        importSources: IMPORT_SOURCES,
        treeshakeCompensation: true,
        runtimeInjection: false,
        dev: false,
        unstable_moduleResolution: { type: "commonJS", rootDir: directory },
    };
    const own = new Map<string, Rule[]>();
    const depended = new Map<string, Rule[]>();
    let added: string | undefined;

    return {
        name: "destack:style",
        enforce: "pre",
        buildStart: async () => {
            // add the rules each dependency's build keeps under its modules, keeping the package's own across rebuilds
            depended.clear();
            for (const dependency of dependencies) {
                let read = RULES.get(dependency);
                if (read === undefined) {
                    read = readRules(dependency, environment);
                    RULES.set(dependency, read);
                }
                for (const [module, rules] of await read) {
                    depended.set(join(dependency, module), rules);
                }
            }
        },
        resolveId: (id) => (withoutQuery(id) === SHEET_ID ? RESOLVED_SHEET_ID : undefined),
        // load the sheet module empty, as the bundle appends the rules to its stylesheet
        load: (id) => (withoutQuery(id) === RESOLVED_SHEET_ID ? "" : undefined),
        transform: {
            filter: { id: MODULE_PATTERN, code: IMPORT_SOURCE_PATTERN },
            handler(code, id) {
                // compile the module's styles, keeping its rules
                const compiled = stylex.transform(withoutQuery(id), code, options);
                const rules = schema.array(Rule).parse(compiled.metadata.stylex);
                if (rules.length > 0) {
                    own.set(id, rules);
                } else {
                    own.delete(id);
                }

                return { code: compiled.code, map: compiled.map ?? null };
            },
        },
        async generateBundle(_options, bundle) {
            // forget the rules of modules that left the graph, as a rebuild transforms only the changed ones
            const graph = new Set(this.getModuleIds());
            for (const module of own.keys()) {
                if (!graph.has(module)) {
                    own.delete(module);
                }
            }

            // keep the build's own rules for the builds depending on it, sorted by module path in the package
            const kept = [...own]
                .map(([module, rules]) => [relative(directory, module), rules] as const)
                .filter(([module]) => !module.startsWith(".."))
                .toSorted(([left], [right]) => (left < right ? -1 : Number(left > right)));
            if (kept.length > 0) {
                this.emitFile({
                    type: "asset",
                    fileName: RULES_FILE,
                    source: JSON.stringify(kept),
                });
            }

            // write every rule into the build's own stylesheet, which the chunk importing the sheet module imports
            const chunks = Object.values(bundle).filter((output) => output.type === "chunk");
            const holder = chunks.find((chunk) => chunk.moduleIds.includes(RESOLVED_SHEET_ID));
            if (holder !== undefined && holder.viteMetadata === undefined) {
                throw new TypeError(
                    `chunk '${holder.fileName}' is invalid: it imports the sheet module without the stylesheets it imports`,
                );
            }
            const imported = holder?.viteMetadata?.importedCss;
            if (imported !== undefined) {
                const css = await cachedSheet([...depended, ...own], cache, environment);
                const reference = this.emitFile({ type: "asset", name: SHEET_FILE, source: css });

                // replace the sheet an earlier build of a warm bundle added, as chunks' metadata outlives it
                for (const chunk of chunks) {
                    if (added !== undefined) {
                        chunk.viteMetadata?.importedCss.delete(added);
                    }
                }
                added = this.getFileName(reference);
                imported.add(added);
            }
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

/** Read the sheet a build's rules make from the cache by their digest, else make and keep it. */
async function cachedSheet(
    modules: readonly (readonly [string, readonly Rule[]])[],
    cache: TransformCache | undefined,
    environment: Pick<Environment<"files">, "files" | "crypto">,
): Promise<string> {
    // key the sheet by StyleX's and Lightning CSS's code and the rules, in module order
    SHEET_IDENTITY ??= Promise.all(
        [import.meta.resolve("@stylexswc/rs-compiler"), import.meta.resolve("lightningcss")].map(
            (url) => environment.files.readTextFile(new URL(url).pathname),
        ),
    );
    const ordered = modules
        .map(([id, rules]) => [packagePath(id), rules] as const)
        .toSorted(([left], [right]) => (left < right ? -1 : Number(left > right)));
    const key = await Digest.of(
        [...(await SHEET_IDENTITY), JSON.stringify(ordered)].join("\0"),
        environment.crypto,
    );

    // take the kept sheet, else make it and keep it for later builds
    const kept = await cache?.read(key);
    if (kept !== undefined) {
        return new TextDecoder().decode(kept);
    }
    const css = sheetOf(ordered.flatMap(([, rules]) => rules));
    await cache?.write(key, new TextEncoder().encode(css));

    return css;
}

/** Write rules as one stylesheet after the base layer, lowered to the baseline browsers. */
function sheetOf(rules: readonly Rule[]): string {
    // leave a sheet without rules empty
    if (rules.length === 0) {
        return "";
    }

    // order the rules into layers, then lower them as every browser stylesheet is
    const collected = stylex.processStylexRules(rules, { useLayers: LAYERS });
    const { lightningcss } = runtimeCss("browser");
    const { code } = transform({
        ...lightningcss,
        filename: "stylex.css",
        code: new TextEncoder().encode(collected),
    });

    return new TextDecoder().decode(code);
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
