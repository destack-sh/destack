import type { Stage } from "@destack/update/release";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { basename, dirname, join, relative, resolve } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import type { Target } from "@destack/update/release";
import { schema } from "@destack/schema";
import { DependencyName } from "@destack/package";
import { PackageLocator } from "@destack/package/locator";
import { modulePlugin } from "./module.ts";
import { toolchainPlugin } from "./toolchain.ts";
import { Platform } from "./platform.ts";
import { read, run } from "./command.ts";

/** The directory beside a distribution's launchers with its programs' bundles. */
export const PROGRAM_DIRECTORY = "program";

/** The runtime beside a distribution's launchers, which runs every program's bundle. */
export const RUNTIME = "bun";

/** The empty Bun configuration every program starts with, so a project's never applies. */
export const CONFIGURATION_FILE = "bunfig.toml";

/** The oldest macOS the launcher runs on, the application's own minimum. */
const MACOS_MINIMUM = "11.0";

/** Where Bun publishes each version's runtimes, with a listing of their SHA-256 digests. */
const RUNTIME_RELEASES = "https://github.com/oven-sh/bun/releases/download";

/** The architecture names the C compiler takes for a macOS target, by the platform's own. */
const COMPILER_ARCHITECTURES = { arm64: "arm64", x64: "x86_64" } as const;

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
    /** Distribution version injected into executable package metadata. */
    version?: string;
    /** The stage of the executable: its identity, issuer and device directory. */
    stage?: Stage;
}

/** Write the options of a program loading native tools, such as rolldown, from their copies. */
export function toolchainOptions(files: FileSystem): Required<Pick<ExecutableOptions, "plugins">> {
    return { plugins: [toolchainPlugin(files)] };
}

/** Build a program as a launcher at `outfile`, with its bundle and the runtime beside it. */
export async function buildExecutable(
    options: ExecutableOptions,
    environment: Environment<"files" | "process" | "host">,
): Promise<void> {
    // refuse another architecture's target off macOS, where the launcher compiles for this computer
    const { files, process, host } = environment;
    const isOwn = options.runtime === `bun-${host.os}-${host.architecture}`;
    if (!isOwn && host.os !== "darwin") {
        throw new Error(`build '${options.target}' on a computer of its own architecture`);
    }

    // bundle the program beside the launcher, its directory assets below the bundles
    const parent = dirname(options.outfile);
    const name = basename(options.outfile);
    const programs = join(parent, PROGRAM_DIRECTORY);
    await files.mkdir(programs, { recursive: true });
    const assets = new Set<string>();
    const peers = await optionalPeers(options.root, environment);
    const external = [...peers, ...(options.external ?? [])];
    const shared = await sharedPlugin(options, external, environment);
    const plugins = [...(shared === undefined ? [] : [shared]), ...(options.plugins ?? [])];
    await bundleProgram(
        { ...options, plugins },
        programs,
        `${name}.js`,
        external,
        assets,
        environment,
    );
    await copyAssets(options.root, assets, join(programs, "asset"), environment);

    // place the runtime and the empty configuration once, then the launcher named after the program
    const runtime = join(parent, RUNTIME);
    if (!(await files.exists(runtime))) {
        // ship the building runtime for its own target
        if (isOwn) {
            await files.copyFile(await files.realPath(process.executable), runtime);
        }
        // ship Bun's release of the same version for another architecture
        else {
            await fetchRuntime(options.runtime, runtime, environment);
        }
        await files.chmod(runtime, 0o755);
        await files.writeTextFile(join(parent, CONFIGURATION_FILE), "");
    }
    await compileLauncher(options.outfile, Platform.of(options.target), environment);
}

/** Fetch a runtime of the building runtime's version from Bun's release, which lists its digest. */
async function fetchRuntime(
    runtime: string,
    outfile: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // read the building runtime's version, which every runtime of a distribution ships at
    const { files, process, network, crypto } = environment;
    const version = (await read(process.executable, ["--version"], environment)).trim();
    const release = `${RUNTIME_RELEASES}/bun-v${version}`;

    // fetch the runtime's archive, which Bun's releases file under aarch64 for arm64
    const asset = runtime.replace("arm64", "aarch64");
    const archive = `${asset}.zip`;
    const [fetched, listing] = await Promise.all([
        network.fetch(new Request(`${release}/${archive}`)),
        network.fetch(new Request(`${release}/SHASUMS256.txt`)),
    ]);
    if (!fetched.ok || !listing.ok) {
        throw new Error(
            `could not fetch runtime '${runtime}' at '${version}': status ${fetched.status} and ${listing.status}`,
        );
    }

    // refuse an archive that arrived with other bytes than its release lists
    const bytes = new Uint8Array(await fetched.arrayBuffer());
    const digest = new Uint8Array(await crypto.digest("SHA-256", bytes)).toHex();
    if (digest !== readDigest(await listing.text(), archive)) {
        throw new Error(
            `runtime '${runtime}' at '${version}' has digest '${digest}', not the listed one`,
        );
    }

    // unpack the archive and take the runtime in it
    const directory = await files.makeTemporaryDirectory("destack-runtime-");
    try {
        const path = join(directory, archive);
        await files.writeFile(path, bytes);
        await run("unzip", ["-q", path, "-d", directory], environment);
        await files.copyFile(join(directory, asset, "bun"), outfile);
    } finally {
        await files.remove(directory, { recursive: true, force: true });
    }
}

/** Read a file's SHA-256 digest from a release's listing, absent for a file it does not list. */
export function readDigest(listing: string, file: string): string | undefined {
    const line = listing.split("\n").find((each) => each.split(/\s+/u)[1] === file);

    return line?.split(/\s+/u)[0];
}

/** Compile the launcher, which runs the bundle named after it on the runtime beside it. */
async function compileLauncher(
    outfile: string,
    platform: Platform,
    environment: Environment<"process" | "host">,
): Promise<void> {
    // build launcher.c for the target's architecture on macOS, with the oldest supported macOS
    const source = fileURLToPath(new URL("./launcher.c", import.meta.url));
    const architecture = COMPILER_ARCHITECTURES[platform.architecture];
    const targeted =
        environment.host.os === "darwin"
            ? ["-arch", architecture, `-mmacosx-version-min=${MACOS_MINIMUM}`]
            : [];
    const child = environment.process.spawn(["cc", "-Os", ...targeted, source, "-o", outfile], {
        stdin: "ignore",
        stdout: "pipe",
        stderr: "pipe",
    });
    const [code, errors] = await Promise.all([child.exited, new Response(child.stderr).text()]);
    if (code !== 0) {
        throw new Error(`the launcher did not compile: '${errors.trim()}'`);
    }
}

/** Bundle the entrypoint into a file, collecting the directory assets its modules read. */
async function bundleProgram(
    options: ExecutableOptions,
    directory: string,
    naming: string,
    external: string[],
    assets: Set<string>,
    environment: Environment<"files">,
): Promise<void> {
    // define the target's operating system for the bundled module graph, its architecture read at runtime
    const { system } = Platform.of(options.target);

    // let source transforms discover assets from the actual bundled module graph
    const bundle = await Bun.build({
        entrypoints: [options.entrypoint],
        target: "bun",
        minify: true,
        outdir: directory,
        naming,
        // write each map apart from its bundle, which names none, as observability reads them and nobody installs them
        sourcemap: "external",
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

    // record the file each specifier of a shared package resolved to in the graph
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
            throw new Error(
                `the program bundles '${specifier}' from '${[...resolved].join(", ")}'`,
            );
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
            throw new Error(`executable asset is a symbolic link: '${path}'`);
        }

        // copy the asset with everything inside it
        await files.copy(asset, join(embedded, relative(root, asset)));
    }
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
    process.env.DESTACK_UNIVERSE ??= ${JSON.stringify(stage.universe)};
    process.env.DESTACK_DIRECTORY ??= destackPath(destackHome(), ${JSON.stringify(stage.directory)});`;
}
