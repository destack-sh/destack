import { cp, lstat, mkdir, mkdtemp, rm } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import type { Target } from "@destack/update/release";
import { nativePlugin } from "./native.ts";
import { modulePlugin } from "./module.ts";

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
    /** Distribution version injected into executable package metadata. */
    version?: string;
    /** Native application identity and default device directory. */
    identity?: "stable" | "nightly" | "dev";
}

/** Bundle modules and their directory assets, then embed them in one executable. */
export async function buildExecutable(options: ExecutableOptions): Promise<void> {
    // isolate generated modules beside the destination until compilation finishes
    await mkdir(dirname(options.outfile), { recursive: true });
    const directory = await mkdtemp(join(dirname(options.outfile), ".executable-"));
    const assets = new Set<string>();
    const platform = options.target.includes("apple")
        ? "darwin"
        : options.target.includes("windows")
          ? "win32"
          : "linux";
    const architecture = options.target.startsWith("aarch64") ? "arm64" : "x64";
    try {
        // let source transforms discover assets from the actual bundled module graph
        const bundle = await Bun.build({
            entrypoints: [options.entrypoint],
            target: "bun",
            minify: true,
            outdir: directory,
            external: ["*.node"],
            naming: "program.js",
            banner: options.identity
                ? `import { homedir as destackHome } from "node:os"; import { join as destackPath } from "node:path";
                process.env.DESTACK_RELEASE_CHANNEL = ${JSON.stringify(options.identity)};
                process.env.DESTACK_DIRECTORY ??= destackPath(destackHome(), ${JSON.stringify(options.identity === "stable" ? ".destack" : `.destack-${options.identity}`)});`
                : undefined,
            plugins: [
                nativePlugin(options.target),
                modulePlugin(options.root, assets, options.version),
                ...(options.plugins ?? []),
            ],
            define: {
                "Bun.isStandaloneExecutable": "true",
                "process.platform": JSON.stringify(platform),
                "process.arch": JSON.stringify(architecture),
            },
        });
        if (!bundle.success) {
            throw new AggregateError(bundle.logs, "executable bundling failed");
        }

        // preserve distinct module paths beneath one embedded asset directory
        const embedded = join(directory, "asset");
        for (const asset of assets) {
            await cp(asset, join(embedded, relative(options.root, asset)), {
                recursive: true,
                filter: async (path) => {
                    if ((await lstat(path)).isSymbolicLink()) {
                        throw new Error(`executable asset is a symbolic link: ${path}`);
                    }

                    return true;
                },
            });
        }

        // compile with explicit assets and no ambient runtime configuration
        const compilation = await Bun.build({
            root: options.root,
            entrypoints: [join(directory, "program.js")],
            target: "bun",
            minify: true,
            sourcemap: "linked",
            compile: {
                target: options.runtime,
                outfile: options.outfile,
                assets: assets.size ? [embedded] : [],
                autoloadDotenv: false,
                autoloadBunfig: false,
                autoloadPackageJson: false,
                autoloadTsconfig: false,
            },
        });
        if (!compilation.success) {
            throw new AggregateError(compilation.logs, "executable compilation failed");
        }
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
}
