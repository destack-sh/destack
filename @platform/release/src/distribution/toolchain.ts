import { cp, mkdir, readFile, realpath } from "node:fs/promises";
import { createRequire } from "node:module";
import { basename, dirname, join, sep } from "node:path";
import type { Target } from "@destack/update/release";
import { schema } from "@destack/schema";
import { parseSync, Visitor } from "rolldown/utils";
import MagicString from "magic-string";
import { selectPlatform } from "./platform.ts";

/** The bundled packages whose modules read their own files at run time, from their copies in the toolchain. */
const RUNTIME_FILES: ReadonlySet<string> = new Set(["vite", "lightningcss"]);

/** The fields of a package.json the toolchain reads. */
const PackageManifest = schema
    .object({
        type: schema.string().exactOptional(),
        dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    })
    .strip();

/** A package the compiler reads at run time, found through the chain of packages depending on it. */
interface ToolchainPackage {
    /** The workspace package, then each package depending on the next, ending in the package itself. */
    readonly chain: readonly string[];
    /** Whether the package's dependencies come along, or only its package.json and destack.json. */
    readonly contents: "dependencies" | "metadata";
}

/** Lay out the compiler's toolchain for a target: the packages it reads at run time, installed flat, and the lint rules. */
export async function buildToolchain(
    root: string,
    target: Target,
    destination: string,
): Promise<void> {
    // copy each toolchain package into one flat node_modules
    const modules = join(destination, "node_modules");
    const copied = new Map<string, string>();
    for (const entry of toolchainPackages(target)) {
        // locate the package through its chain of dependents
        const [workspace, ...names] = entry.chain;
        let directory = join(root, workspace ?? "");
        for (const name of names) {
            directory = await locate(name, directory);
        }
        const name = names.at(-1) ?? "";

        // copy only the manifests of a metadata package
        if (entry.contents === "metadata") {
            await mkdir(join(modules, name), { recursive: true });
            for (const file of ["package.json", "destack.json"]) {
                await cp(join(directory, file), join(modules, name, file));
            }
        }
        // copy the package with its dependencies and refuse two copies of one name
        else {
            await copyPackage(name, directory, modules, copied);
        }
    }

    // bundle the lint rules into one module
    const lint = await Bun.build({
        entrypoints: [join(root, "@destack/check/src/lint/index.ts")],
        target: "bun",
        outdir: destination,
        naming: "lint.js",
    });
    if (!lint.success) {
        throw new AggregateError(lint.logs, "lint rule bundling failed");
    }
}

/** Point the bundled modules of packages reading their own files at run time to their copies in the toolchain beside the executable. */
export function toolchainPlugin(): Bun.BunPlugin {
    return {
        name: "destack-executable-toolchain",
        setup(build) {
            build.onLoad({ filter: /[\\/]node_modules[\\/].+\.[cm]?js$/u }, async ({ path }) => {
                // leave every package but those reading their own files
                const installed = installation(path);
                if (installed === undefined || !RUNTIME_FILES.has(installed.name)) {
                    return;
                }

                // locate the module's copy relative to the executable
                const copy = `toolchain/node_modules/${installed.name}/${installed.path}`;
                const url = `new URL(${JSON.stringify(copy)}, Bun.pathToFileURL(process.execPath)).href`;
                const source = await readFile(path, "utf8");

                // require from the copy in a CommonJS module
                if (path.endsWith(".cjs") || !(await isModulePackage(installed.directory))) {
                    return {
                        contents: `var require = process.getBuiltinModule("node:module").createRequire(${url});\n${source}`,
                        loader: "js",
                    };
                }

                // read the copy's URL as the module's own
                return { contents: replaceModuleUrl(path, source, url), loader: "js" };
            });
        },
    };
}

/** List the packages the compiler reads at run time for a target, with the platform packages of its native tools. */
function toolchainPackages(target: Target): readonly ToolchainPackage[] {
    // name the target as package.json fields and napi-rs packages name platforms
    const { system: os, architecture: cpu } = selectPlatform(target);
    const abi = /-(gnu|msvc)$/u.exec(target)?.[1];
    const native = `${os}-${cpu}${abi === undefined ? "" : `-${abi}`}`;
    const build = "@destack/build";
    const check = "@destack/check";

    return [
        // the TypeScript compiler
        {
            chain: [build, "typescript", `@typescript/typescript-${os}-${cpu}`],
            contents: "dependencies",
        },
        // the rolldown binding
        { chain: [build, "rolldown", `@rolldown/binding-${native}`], contents: "dependencies" },
        // the vite client files
        { chain: [build, "vite"], contents: "dependencies" },
        // the CSS transformer
        {
            chain: [build, "vite", "lightningcss", `lightningcss-${native}`],
            contents: "dependencies",
        },

        // the runtime declarations
        { chain: [build, "@types/bun"], contents: "dependencies" },
        { chain: [build, "@cloudflare/workers-types"], contents: "dependencies" },

        // the packages builds name
        { chain: [build, "@destack/test"], contents: "metadata" },
        { chain: [build, "@destack/resource"], contents: "metadata" },

        // the linter, the formatter and the type-aware linter
        { chain: [check, "oxlint"], contents: "dependencies" },
        { chain: [check, "oxlint", `@oxlint/binding-${native}`], contents: "dependencies" },
        { chain: [check, "oxfmt"], contents: "dependencies" },
        { chain: [check, "oxfmt", `@oxfmt/binding-${native}`], contents: "dependencies" },
        {
            chain: [check, "oxlint-tsgolint", `@oxlint-tsgolint/${os}-${cpu}`],
            contents: "dependencies",
        },
    ];
}

/** Copy a package and its dependencies into a flat node_modules, refusing two installations of one name. */
async function copyPackage(
    name: string,
    directory: string,
    modules: string,
    copied: Map<string, string>,
): Promise<void> {
    // copy each installation once, refusing another installation of the same name
    const real = await realpath(directory);
    const previous = copied.get(name);
    if (previous === real) {
        return;
    } else if (previous !== undefined) {
        throw new Error(
            `the toolchain needs two installations of ${name}: ${previous} and ${real}`,
        );
    }
    copied.set(name, real);

    // copy the package's own files, leaving out its installed dependencies
    await cp(real, join(modules, name), {
        recursive: true,
        dereference: true,
        filter: (path) => basename(path) !== "node_modules",
    });

    // copy its dependencies, resolved from the package
    const manifest = PackageManifest.parse(
        JSON.parse(await readFile(join(real, "package.json"), "utf8")),
    );
    for (const dependency of Object.keys(manifest.dependencies ?? {})) {
        await copyPackage(dependency, await locate(dependency, real), modules, copied);
    }
}

/** Find an installed package's directory from a package directory. */
async function locate(name: string, from: string): Promise<string> {
    return realpath(
        dirname(createRequire(join(from, "package.json")).resolve(`${name}/package.json`)),
    );
}

/** Read the installed package containing a module: its name, directory and the module's path within it. */
function installation(
    path: string,
): { readonly name: string; readonly directory: string; readonly path: string } | undefined {
    // take the package after the last node_modules segment, scoped or not
    const marker = `${sep}node_modules${sep}`;
    const start = path.lastIndexOf(marker);
    if (start < 0) {
        return undefined;
    }
    const segments = path.slice(start + marker.length).split(sep);
    const length = segments[0]?.startsWith("@") === true ? 2 : 1;
    const name = segments.slice(0, length).join("/");

    return {
        name,
        directory: path.slice(0, start + marker.length) + segments.slice(0, length).join(sep),
        path: segments.slice(length).join("/"),
    };
}

/** Report whether a package's modules are ECMAScript modules. */
async function isModulePackage(directory: string): Promise<boolean> {
    const manifest = PackageManifest.parse(
        JSON.parse(await readFile(join(directory, "package.json"), "utf8")),
    );

    return manifest.type === "module";
}

/** Replace each `import.meta.url` of a module with another URL expression. */
function replaceModuleUrl(path: string, source: string, url: string): string {
    // find each import.meta.url
    const parsed = parseSync(path, source);
    if (parsed.errors.length > 0) {
        throw new Error(`invalid toolchain module: ${path}`);
    }
    const output = new MagicString(source);
    new Visitor({
        MemberExpression(node) {
            if (
                node.object.type === "MetaProperty" &&
                node.object.meta.name === "import" &&
                node.object.property.name === "meta" &&
                !node.computed &&
                node.property.type === "Identifier" &&
                node.property.name === "url"
            ) {
                output.overwrite(node.start, node.end, `(${url})`);
            }
        },
    }).visit(parsed.program);

    return output.toString();
}
