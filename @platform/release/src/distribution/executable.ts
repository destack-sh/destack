import type { Stage } from "@destack/update/release";
import { createRequire } from "node:module";
import { basename, dirname, join, relative, resolve } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import type { Target } from "@destack/update/release";
import { schema } from "@destack/schema";
import { DependencyName } from "@destack/package";
import { PackageLocator } from "@destack/package/locator";
import { modulePlugin } from "./module.ts";
import { toolchainPlugin } from "./toolchain.ts";
import { Platform } from "./platform.ts";

/** The directory below the checkout keeping each compiled executable by its inputs' digest. */
const EXECUTABLE_CACHE = "dist/.executables";

/** The module whose table of shared modules a compiled program's build generates. */
const SHARED_MODULE = /[\\/]@destack[\\/]package[\\/]src[\\/]bun[\\/]shared\.ts$/u;

/** The modules of a bundled graph with the specifiers each imports, as Bun's metafile has it. */
const ModuleGraph = schema
    .object({
        inputs: schema.record(
            schema.string(),
            schema
                .object({
                    imports: schema.array(
                        schema
                            .object({
                                path: schema.string(),
                                original: schema.string().exactOptional(),
                                external: schema.boolean().exactOptional(),
                            })
                            .strip(),
                    ),
                })
                .strip(),
        ),
    })
    .strip();

/** The peer metadata a package manifest declares. */
const PeerManifest = schema
    .object({
        peerDependenciesMeta: schema.record(
            schema.string(),
            schema.object({ optional: schema.boolean().exactOptional() }).strip(),
        ),
    })
    .strip();

/** Sources, assets and target selected for one standalone executable. */
export interface ExecutableOptions {
    /** Repository root used to retain distinct package asset directories. */
    root: string;
    /** Absolute executable source entrypoint. */
    entrypoint: string;
    /** Absolute destination executable. */
    outfile: string;
    /** Native distribution target. */
    target: Target;
    /** Corresponding Bun compilation target. */
    runtime: Bun.Build.CompileTarget;
    /** Additional source transforms required by the application. */
    plugins?: Bun.BunPlugin[];
    /** Packages the executable never loads, left out of its bundle. */
    external?: string[];
    /** What the executable loads from disk: its bundle, packages by their manifests, or sources. */
    loading: "bundle" | "packages" | "sources";
    /** Distribution version injected into executable package metadata. */
    version?: string;
    /** The stage of the executable: its identity, issuer and device directory. */
    stage?: Stage;
}

/** Write the options of a program loading native tools, such as rolldown, from its toolchain. */
export function toolchainOptions(
    files: FileSystem,
): Required<Pick<ExecutableOptions, "loading" | "plugins">> {
    return { loading: "packages", plugins: [toolchainPlugin(files)] };
}

/** Write the compiler's options: sources from disk, native tools from its toolchain. */
export function compilerOptions(
    files: FileSystem,
): Pick<ExecutableOptions, "external" | "loading" | "plugins"> {
    return { ...toolchainOptions(files), external: ["oxfmt"], loading: "sources" };
}

/** Embed bundled modules and their directory assets in one executable. */
export async function buildExecutable(
    options: ExecutableOptions,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // isolate generated modules beside the destination until compilation finishes
    const { files } = environment;
    const parent = dirname(options.outfile);
    await files.mkdir(parent, { recursive: true });
    const directory = await files.makeTemporaryDirectory(".executable-", { directory: parent });
    const assets = new Set<string>();
    const peers = await optionalPeers(options.root, environment);
    const external = [...peers, ...(options.external ?? [])];
    try {
        // generate the table of modules the program shares, then bundle the program with it
        const shared = await sharedPlugin(options, external, environment);
        const plugins = [...(shared === undefined ? [] : [shared]), ...(options.plugins ?? [])];
        await bundleProgram({ ...options, plugins }, directory, external, assets, environment);
        const embedded = join(directory, "asset");
        await copyAssets(options.root, assets, embedded, environment);
        const embeddings = assets.size ? [embedded] : [];

        // take the executable compiled from the same bundle, assets and options, else compile and keep it
        const digest = await programDigest(options, directory, external, embeddings, environment);
        const name = basename(options.outfile);
        const cached = join(options.root, EXECUTABLE_CACHE, digest);
        if (!(await files.exists(join(cached, name)))) {
            const staging = await files.makeTemporaryDirectory(".compiling-", {
                directory: parent,
            });
            const outfile = join(staging, name);
            await compileProgram({ ...options, outfile }, directory, external, embeddings);
            await files.mkdir(dirname(cached), { recursive: true });
            await files.rename(staging, cached).catch(async (error: unknown) => {
                // keep the copy another build kept first
                await files.remove(staging, { recursive: true, force: true });
                if (!(await files.exists(join(cached, name)))) {
                    throw error;
                }
            });
        }
        await files.copyFile(join(cached, name), options.outfile);
        await files.copyFile(join(cached, `${name}.map`), `${options.outfile}.map`);
    } finally {
        await files.remove(directory, { recursive: true, force: true });
    }
}

/** Bundle the entrypoint into `program.js`, collecting the directory assets its modules read. */
async function bundleProgram(
    options: ExecutableOptions,
    directory: string,
    external: string[],
    assets: Set<string>,
    environment: Environment<"files">,
): Promise<void> {
    // define the target platform for the bundled module graph
    const { system, architecture } = Platform.of(options.target);

    // let source transforms discover assets from the actual bundled module graph
    const bundle = await Bun.build({
        entrypoints: [options.entrypoint],
        target: "bun",
        minify: true,
        outdir: directory,
        naming: "program.js",
        external,
        ...(options.stage !== undefined && { banner: channelBanner(options.stage) }),
        // let the caller's plugins load their modules first, as the module plugin answers every other
        plugins: [
            ...(options.plugins ?? []),
            modulePlugin(options.root, assets, environment.files, options.version),
        ],
        define: {
            "Bun.isStandaloneExecutable": "true",
            "process.platform": JSON.stringify(system),
            "process.arch": JSON.stringify(architecture),
        },
    });
    if (!bundle.success) {
        throw new AggregateError(bundle.logs, "executable bundling failed");
    }
}

/** Generate the modules a program shares with the code it loads, when its graph reads the table. */
async function sharedPlugin(
    options: ExecutableOptions,
    external: readonly string[],
    environment: Environment<"files" | "process">,
): Promise<Bun.BunPlugin | undefined> {
    // read the program's module graph, leaving a program that reads no table
    const bundle = await Bun.build({
        entrypoints: [options.entrypoint],
        target: "bun",
        external: [...external],
        metafile: true,
        throw: false,
    });
    if (!bundle.success) {
        throw new AggregateError(bundle.logs, "executable graph failed");
    }
    const graph = ModuleGraph.parse(bundle.metafile);
    if (!Object.keys(graph.inputs).some((input) => SHARED_MODULE.test(input))) {
        return undefined;
    }

    // keep the file each specifier of a shared package resolved to, which the graph imports
    const locator = new PackageLocator(environment.files);
    const shared = await locator.shared(dirname(options.entrypoint));
    const { workingDirectory } = environment.process;
    const files = new Map<string, Set<string>>();
    for (const imported of Object.values(graph.inputs).flatMap((input) => input.imports)) {
        const specifier = imported.original;
        if (
            specifier !== undefined &&
            imported.external !== true &&
            shared.has(DependencyName.of(specifier))
        ) {
            const resolved = files.get(specifier) ?? new Set<string>();
            files.set(specifier, resolved.add(resolve(workingDirectory, imported.path)));
        }
    }

    // import each from that file, which the graph already holds, refusing two copies of one
    const entries = [...files].toSorted(([left], [right]) => left.localeCompare(right));
    const lines = entries.map(([specifier, resolved], index) => {
        const [file, ...others] = resolved;
        if (file === undefined || others.length > 0) {
            throw new Error(`the program bundles ${specifier} from ${[...resolved].join(", ")}`);
        }

        return {
            imported: `import * as shared${index} from ${JSON.stringify(file)};`,
            entry: `    ${JSON.stringify(specifier)}: async () => shared${index},`,
        };
    });
    const imports = lines.map((line) => line.imported).join("\n");
    const table = lines.map((line) => line.entry).join("\n");
    const contents = `${imports}\nexport const SHARED_MODULES = {\n${table}\n};\n`;

    return {
        name: "destack-executable-shared",
        setup(build) {
            build.onLoad({ filter: SHARED_MODULE }, () => ({ contents, loader: "js" }));
        },
    };
}

/** Copy each asset directory below one embedded directory at its path relative to the root. */
async function copyAssets(
    root: string,
    assets: Set<string>,
    embedded: string,
    environment: Environment<"files">,
): Promise<void> {
    // preserve distinct module paths beneath one embedded asset directory
    const { files } = environment;
    for (const asset of assets) {
        // refuse a symbolic link anywhere in the asset
        const isLink = (await files.lstat(asset)).kind === "symlink";
        const entries = await files.readDirectory(asset, { recursive: true });
        const link = entries.find((entry) => entry.kind === "symlink");
        if (isLink || link !== undefined) {
            const path = link === undefined ? asset : join(asset, link.name);
            throw new Error(`executable asset is a symbolic link: ${path}`);
        }

        // copy the asset with everything inside it
        await files.copy(asset, join(embedded, relative(root, asset)));
    }
}

/** Compile the bundled program and its embedded assets into the destination executable. */
async function compileProgram(
    options: ExecutableOptions,
    directory: string,
    external: string[],
    assets: string[],
): Promise<void> {
    // compile without ambient runtime configuration, resolving packages and sources from disk as the executable loads them
    const compiled = await Bun.build({
        entrypoints: [join(directory, "program.js")],
        root: options.root,
        minify: true,
        sourcemap: "linked",
        external,
        compile: {
            target: options.runtime,
            outfile: options.outfile,
            assets,
            autoloadDotenv: false,
            autoloadBunfig: false,
            autoloadPackageJson: options.loading !== "bundle",
            autoloadTsconfig: options.loading === "sources",
        },
    });
    if (!compiled.success) {
        throw new AggregateError(compiled.logs, "executable compilation failed");
    }
}

/** Digest what an executable compiles from: its bundle, assets, options and Bun. */
async function programDigest(
    options: ExecutableOptions,
    directory: string,
    external: readonly string[],
    embeddings: readonly string[],
    environment: Environment<"files">,
): Promise<string> {
    // hash the options and Bun's revision, then the bundle
    const { files } = environment;
    const hasher = new Bun.CryptoHasher("sha256");
    const { target, runtime, loading, version } = options;
    hasher.update(
        JSON.stringify({ target, runtime, loading, version, external, bun: Bun.revision }),
    );
    hasher.update(await files.readFile(join(directory, "program.js")));

    // hash each embedded asset by its path and contents, in path order
    for (const embedded of embeddings) {
        const entries = await files.readDirectory(embedded, { recursive: true });
        const paths = entries.filter((entry) => entry.kind === "file").map((entry) => entry.name);
        for (const path of paths.toSorted()) {
            hasher.update(path);
            hasher.update(await files.readFile(join(embedded, path)));
        }
    }

    return hasher.digest("hex");
}

/** List vite's optional peers and their subpaths. */
async function optionalPeers(root: string, environment: Environment<"files">): Promise<string[]> {
    // read the peers of the vite @destack/build resolves
    const require = createRequire(join(root, "@destack/build/package.json"));
    const text = await environment.files.readTextFile(require.resolve("vite/package.json"));
    const manifest = PeerManifest.parse(JSON.parse(text));
    const names = Object.entries(manifest.peerDependenciesMeta)
        .filter(([, peer]) => peer.optional === true)
        .map(([name]) => name);

    return names.flatMap((name) => [name, `${name}/*`]);
}

/** Write the module banner that sets a stage's name, issuer and device directory. */
function channelBanner(stage: Stage): string {
    return `import { homedir as destackHome } from "node:os"; import { join as destackPath } from "node:path";
    process.env.DESTACK_STAGE = ${JSON.stringify(stage.name)};
    process.env.DESTACK_ISSUER ??= ${JSON.stringify(stage.issuer)};
    process.env.DESTACK_DIRECTORY ??= destackPath(destackHome(), ${JSON.stringify(stage.directory)});`;
}
