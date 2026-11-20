import type { Profile } from "@destack/update/release";
import { createRequire } from "node:module";
import { basename, join, resolve } from "node:path";
import type { Environment } from "@destack/environment";
import type { Target } from "@destack/update/release";
import { schema } from "@destack/schema";
import { DependencyName } from "@destack/package";
import { PackageLocator } from "@destack/package/locator";
import { modulePlugin } from "./module.ts";
import { toolchainPlugin } from "./toolchain.ts";
import { Platform } from "./platform.ts";

/** The shared modules table a compiled program's build generates. */
const SHARED_MODULE = /[\\/]@destack[\\/]package[\\/]src[\\/]bun[\\/]shared\.ts$/u;

/** A bundled graph's modules and imports, as Bun's metafile has them. */
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

/** A distribution executable's sources and target. */
export interface ExecutableOptions {
    /** The checkout. */
    root: string;
    /** Absolute executable source entrypoint. */
    entrypoint: string;
    /** The directory of the package whose shared modules the table lists. */
    sharing: string;
    /** Absolute destination executable. */
    outfile: string;
    /** The directory for the chunks' source maps. */
    maps: string;
    /** Native distribution target. */
    target: Target;
    /** Corresponding Bun compilation target. */
    runtime: Bun.Build.CompileTarget;
    /** Packages left out of the bundle. */
    external?: string[];
    /** The distribution version stamped into package metadata. */
    version?: string;
    /** The executable's profile. */
    profile?: Profile;
}

/** Compile an entrypoint into one executable with bytecode chunks. */
export async function buildExecutable(
    options: ExecutableOptions,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // leave out the optional peers, and generate the shared modules table
    const peers = await optionalPeers(options.root, environment);
    const external = [...peers, ...(options.external ?? [])];
    const shared = await sharedPlugin(options, external, environment);

    // compile the graph for the target
    const { system } = Platform.of(options.target);
    const bundle = await Bun.build({
        entrypoints: [options.entrypoint],
        target: "bun",
        format: "esm",
        minify: true,
        splitting: true,
        bytecode: true,
        sourcemap: "external",
        external,
        // ignore a project's .env, bunfig and tsconfig
        compile: {
            target: options.runtime,
            outfile: options.outfile,
            autoloadDotenv: false,
            autoloadBunfig: false,
            autoloadTsconfig: false,
            autoloadPackageJson: true,
        },
        ...(options.profile !== undefined && { banner: profileBanner(options.profile) }),
        plugins: [
            ...(shared === undefined ? [] : [shared]),
            toolchainPlugin(options.target, environment.files),
            modulePlugin(environment.files, options.version),
        ],
        define: {
            "Bun.isStandaloneExecutable": "true",
            "process.platform": JSON.stringify(system),
        },
        throw: false,
    });
    if (!bundle.success) {
        throw new AggregateError(bundle.logs, "executable compilation failed");
    }

    // move the source maps out of bin
    const { files } = environment;
    await files.mkdir(options.maps, { recursive: true });
    for (const output of bundle.outputs.filter((each) => each.kind === "sourcemap")) {
        await files.rename(output.path, join(options.maps, basename(output.path)));
    }
}

/** Generate the modules a program shares with the code it loads. */
async function sharedPlugin(
    options: ExecutableOptions,
    external: readonly string[],
    environment: Environment<"files" | "process">,
): Promise<Bun.BunPlugin | undefined> {
    // read the module graph, skipping a program without the table
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

    // record each shared specifier's resolved file
    const locator = new PackageLocator(environment.files);
    const shared = await locator.shared(options.sharing);
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

    // load each on demand, refusing two copies of one
    const entries = [...files].toSorted(([left], [right]) => left.localeCompare(right));
    const lines = entries.map(([specifier, resolved]) => {
        const [file, ...others] = resolved;
        if (file === undefined || others.length > 0) {
            throw new Error(
                `the program bundles '${specifier}' from '${[...resolved].join(", ")}'`,
            );
        }

        return `    ${JSON.stringify(specifier)}: () => import(${JSON.stringify(file)}),`;
    });
    const contents = `export const SHARED_MODULES = {\n${lines.join("\n")}\n};\n`;

    return {
        name: "destack-executable-shared",
        setup(build) {
            build.onLoad({ filter: SHARED_MODULE }, () => ({ contents, loader: "js" }));
        },
    };
}

/** List vite's optional peers and their subpaths. */
async function optionalPeers(root: string, environment: Environment<"files">): Promise<string[]> {
    // read the peers of build's vite
    const require = createRequire(join(root, "@destack/build/package.json"));
    const text = await environment.files.readTextFile(require.resolve("vite/package.json"));
    const manifest = PeerManifest.parse(JSON.parse(text));
    const names = Object.entries(manifest.peerDependenciesMeta)
        .filter(([, peer]) => peer.optional === true)
        .map(([name]) => name);

    return names.flatMap((name) => [name, `${name}/*`]);
}

/** Write the banner setting the profile's variables. */
function profileBanner(profile: Profile): string {
    const directory = `process.getBuiltinModule("node:path").join(process.getBuiltinModule("node:os").homedir(), ${JSON.stringify(profile.directory)})`;

    return `process.env.DESTACK_PROFILE = ${JSON.stringify(profile.name)};
    process.env.DESTACK_UNIVERSE_ORIGIN ??= ${JSON.stringify(profile.universe)};
    process.env.DESTACK_DIRECTORY ??= ${directory};`;
}
