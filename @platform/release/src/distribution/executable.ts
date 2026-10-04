import type { ReleaseChannel } from "@destack/daemon/process";
import { cp, lstat, mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, relative } from "node:path";
import type { Target } from "@destack/update/release";
import { schema } from "@destack/schema";
import { modulePlugin } from "./module.ts";
import { toolchainPlugin } from "./toolchain.ts";
import { selectPlatform } from "./platform.ts";

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
    /** Whether the executable loads source modules from disk with their package.json and tsconfig.json. */
    isLoadingSources?: boolean;
    /** Distribution version injected into executable package metadata. */
    version?: string;
    /** The release channel whose identity, issuer and device directory the executable carries. */
    channel?: ReleaseChannel;
}

/** The options of the compiler, which loads package sources from disk and native tools from its toolchain. */
export const COMPILER = {
    external: ["oxfmt"],
    isLoadingSources: true,
    plugins: [toolchainPlugin()],
} satisfies Pick<ExecutableOptions, "external" | "isLoadingSources" | "plugins">;

/** Bundle modules and their directory assets, then embed them in one executable. */
export async function buildExecutable(options: ExecutableOptions): Promise<void> {
    // isolate generated modules beside the destination until compilation finishes
    await mkdir(dirname(options.outfile), { recursive: true });
    const directory = await mkdtemp(join(dirname(options.outfile), ".executable-"));
    const assets = new Set<string>();
    const external = [...(await optionalPeers(options.root)), ...(options.external ?? [])];
    try {
        // bundle the program before embedding the assets its module graph references
        await bundleProgram(options, directory, external, assets);
        const embedded = join(directory, "asset");
        await copyAssets(options.root, assets, embedded);
        await compileProgram(options, directory, external, assets.size ? [embedded] : []);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
}

/** Bundle the entrypoint into `program.js` and collect the directory assets its modules reference. */
async function bundleProgram(
    options: ExecutableOptions,
    directory: string,
    external: string[],
    assets: Set<string>,
): Promise<void> {
    // define the target platform for the bundled module graph
    const { system, architecture } = selectPlatform(options.target);

    // let source transforms discover assets from the actual bundled module graph
    const bundle = await Bun.build({
        entrypoints: [options.entrypoint],
        target: "bun",
        minify: true,
        outdir: directory,
        naming: "program.js",
        external,
        ...(options.channel !== undefined && { banner: channelBanner(options.channel) }),
        plugins: [modulePlugin(options.root, assets, options.version), ...(options.plugins ?? [])],
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

/** Copy each asset directory below one embedded directory at its path relative to the root. */
async function copyAssets(root: string, assets: Set<string>, embedded: string): Promise<void> {
    // preserve distinct module paths beneath one embedded asset directory
    for (const asset of assets) {
        await cp(asset, join(embedded, relative(root, asset)), {
            recursive: true,
            filter: async (path) => {
                if ((await lstat(path)).isSymbolicLink()) {
                    throw new Error(`executable asset is a symbolic link: ${path}`);
                }

                return true;
            },
        });
    }
}

/** Compile the bundled program and its embedded assets into the destination executable. */
async function compileProgram(
    options: ExecutableOptions,
    directory: string,
    external: string[],
    assets: string[],
): Promise<void> {
    // compile with explicit assets and without ambient runtime configuration
    const isLoadingSources = options.isLoadingSources === true;
    const compilation = await Bun.build({
        root: options.root,
        entrypoints: [join(directory, "program.js")],
        target: "bun",
        minify: true,
        sourcemap: "linked",
        external,
        compile: {
            target: options.runtime,
            outfile: options.outfile,
            assets,
            autoloadDotenv: false,
            autoloadBunfig: false,
            // load the package and TypeScript configuration of loaded sources
            autoloadPackageJson: isLoadingSources,
            autoloadTsconfig: isLoadingSources,
        },
    });
    if (!compilation.success) {
        throw new AggregateError(compilation.logs, "executable compilation failed");
    }
}

/** List vite's optional peers and their subpaths. */
async function optionalPeers(root: string): Promise<string[]> {
    // read the peers of the vite @destack/build resolves
    const require = createRequire(join(root, "@destack/build/package.json"));
    const manifest = PeerManifest.parse(
        JSON.parse(await readFile(require.resolve("vite/package.json"), "utf8")),
    );
    const names = Object.entries(manifest.peerDependenciesMeta)
        .filter(([, peer]) => peer.optional === true)
        .map(([name]) => name);

    return names.flatMap((name) => [name, `${name}/*`]);
}

/** Write the module banner that sets a channel's name, issuer and device directory. */
function channelBanner(channel: ReleaseChannel): string {
    return `import { homedir as destackHome } from "node:os"; import { join as destackPath } from "node:path";
    process.env.DESTACK_RELEASE_CHANNEL = ${JSON.stringify(channel.name)};
    process.env.DESTACK_ISSUER ??= ${JSON.stringify(channel.issuer)};
    process.env.DESTACK_DIRECTORY ??= destackPath(destackHome(), ${JSON.stringify(channel.directory)});`;
}
