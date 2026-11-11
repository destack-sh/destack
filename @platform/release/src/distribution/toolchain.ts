import { createRequire } from "node:module";
import { basename, dirname, join, sep } from "node:path";
import type { Architecture, Environment, FileSystem } from "@destack/environment";
import { Distribution, Manifest } from "@destack/update";
import type { Target } from "@destack/update/release";
import { schema } from "@destack/schema";
import { TOOLCHAIN_PACKAGES } from "@destack/package/build";
import { parseSync, Visitor } from "rolldown/utils";
import MagicString from "magic-string";
import { Platform } from "./platform.ts";
import { run } from "./command.ts";

/** The bundled packages reading their own files at run time, by where their copies are. */
const RUNTIME_FILES: ReadonlyMap<string, PackagePlace> = new Map([
    ["rolldown", "resources"],
    ["vite", "toolchain"],
    ["lightningcss", "toolchain"],
]);

/** The expression a rewritten module reads its toolchain directory from, refusing a missing one. */
const TOOLCHAIN_EXPRESSION = `(process.env.DESTACK_TOOLCHAIN ?? (() => { throw new Error("DESTACK_TOOLCHAIN is not set"); })())`;

/** The fields of a package.json the toolchain reads. */
const PackageManifest = schema
    .object({
        type: schema.string().exactOptional(),
        dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    })
    .strip();

/** Where a package goes: beside the programs, which load it at run time, or in the toolchain. */
type PackagePlace = "resources" | "toolchain";

/** A package the compiler reads at run time, found through its chain of dependents. */
interface ToolchainPackage {
    /** The chain of packages from the workspace package to the package itself. */
    readonly chain: readonly string[];
    /** Whether the package's dependencies come along, or only its package.json and destack.json. */
    readonly contents: "dependencies" | "metadata";
    /** Where the package goes. */
    readonly place: PackagePlace;
}

/** Lay out the packages beside the programs, and the compiler's packages and lint rules. */
export async function buildToolchain(
    root: string,
    target: Target,
    directories: Readonly<Record<PackagePlace, string>>,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // copy each package into its place's node_modules, the programs' first, so the toolchain leaves theirs out
    const { files } = environment;
    const nodeModules = new NodeModules(files);

    // refuse a toolchain leaving out a package the builds take from it
    const listed = toolchainPackages(target);
    const shipped = new Set(listed.flatMap((entry) => entry.chain));
    const missing = [...TOOLCHAIN_PACKAGES].filter((name) => !shipped.has(name));
    if (missing.length > 0) {
        throw new Error(
            `the toolchain leaves out '${missing.join(", ")}', which builds take from it`,
        );
    }

    // order the packages, the programs' first
    const packages = listed.toSorted(
        (left, right) => Number(left.place === "toolchain") - Number(right.place === "toolchain"),
    );
    for (const entry of packages) {
        // locate the package through its chain of dependents
        const [workspace, ...names] = entry.chain;
        let directory = join(root, workspace ?? "");
        for (const name of names) {
            directory = await locate(name, directory, files);
        }
        const name = names.at(-1) ?? "";
        const modules = join(directories[entry.place], "node_modules");

        // copy only the manifests of a metadata package
        if (entry.contents === "metadata") {
            await files.mkdir(join(modules, name), { recursive: true });
            for (const file of ["package.json", "destack.json"]) {
                await files.copyFile(join(directory, file), join(modules, name, file));
            }
        }
        // copy the package with its dependencies
        else {
            await nodeModules.copy(name, directory, modules);
        }
    }

    // bundle the lint rules into one module
    await run(
        environment.process.executable,
        [
            "build",
            join(root, "@destack/check/src/lint/index.ts"),
            "--target=bun",
            `--outfile=${join(directories.toolchain, "lint.js")}`,
        ],
        environment,
    );
}

/** Write a toolchain's manifest into a release's resources, keeping recorded modes. */
export async function writeToolchainManifest(
    toolchain: string,
    resources: string,
    architecture: Architecture,
    environment: Environment<"files">,
): Promise<void> {
    // read the toolchain's files, and the manifest written before, such as the build's before signing
    const { files } = environment;
    const read = await Manifest.read(toolchain, environment);
    const path = join(resources, Distribution.toolchainManifest(architecture));
    const written = (await files.exists(path))
        ? Manifest.parse(await files.readTextFile(path))
        : read;

    // keep each file's recorded mode, as artifacts between release jobs drop the modes
    await files.writeTextFile(path, read.withModesOf(written).encode());
}

/** Point modules reading their own files at run time to their copies. */
export function toolchainPlugin(files: FileSystem): Bun.BunPlugin {
    return {
        name: "destack-executable-toolchain",
        setup(build) {
            build.onLoad({ filter: /[\\/]node_modules[\\/].+\.[cm]?js$/u }, async ({ path }) => {
                // leave every package but those reading their own files
                const installed = installation(path);
                const place =
                    installed === undefined ? undefined : RUNTIME_FILES.get(installed.name);
                if (installed === undefined || place === undefined) {
                    return;
                }

                // locate the module's copy beside the programs, one directory above the bundle, else in the named toolchain
                const copy = `node_modules/${installed.name}/${installed.path}`;
                const url =
                    place === "resources"
                        ? `new URL(${JSON.stringify(`../${copy}`)}, Bun.pathToFileURL(Bun.main)).href`
                        : `Bun.pathToFileURL(${TOOLCHAIN_EXPRESSION} + ${JSON.stringify(`/${copy}`)}).href`;
                const source = await files.readTextFile(path);

                // require from the copy in a CommonJS module, its source taking that require as a parameter
                const isCommonJs =
                    path.endsWith(".cjs") ||
                    (path.endsWith(".js") && !(await isModulePackage(installed.directory, files)));
                if (isCommonJs) {
                    const copied = `process.getBuiltinModule("node:module").createRequire(${url})`;

                    return {
                        contents: `(function (require) {\n${source}\n})(${copied});`,
                        loader: "js",
                    };
                }

                // read the copy's URL as the module's own
                return {
                    contents: replaceModuleUrl(path, source, url),
                    loader: "js",
                };
            });
        },
    };
}

/** List the packages the compiler reads at run time for a target, with its native tools. */
function toolchainPackages(target: Target): readonly ToolchainPackage[] {
    // name the target as package.json fields and napi-rs packages name platforms
    const { system: os, architecture: cpu } = Platform.of(target);
    const abi = /-(gnu|msvc)$/u.exec(target)?.[1];
    const native = `${os}-${cpu}${abi === undefined ? "" : `-${abi}`}`;
    const build = "@destack/build";
    const check = "@destack/check";
    const view = "@destack/view";

    return [
        // the TypeScript compiler
        {
            chain: [build, "typescript", `@typescript/typescript-${os}-${cpu}`],
            contents: "dependencies",
            place: "toolchain",
        },
        // the bundler and its binding, which the programs load at run time
        { chain: [build, "rolldown"], contents: "dependencies", place: "resources" },
        {
            chain: [build, "rolldown", `@rolldown/binding-${native}`],
            contents: "dependencies",
            place: "resources",
        },
        // the vite client files
        { chain: [build, "vite"], contents: "dependencies", place: "toolchain" },
        // the CSS transformer
        {
            chain: [build, "vite", "lightningcss", `lightningcss-${native}`],
            contents: "dependencies",
            place: "toolchain",
        },

        // the view compiler and the image renderer, with their native bindings
        { chain: [view, "@solidjs/vite-plugin"], contents: "dependencies", place: "toolchain" },
        {
            chain: [
                view,
                "@solidjs/vite-plugin",
                "@solidjs/compiler",
                `@solidjs/compiler-${native}`,
            ],
            contents: "dependencies",
            place: "toolchain",
        },
        { chain: [view, "sharp"], contents: "dependencies", place: "toolchain" },
        {
            chain: [view, "sharp", `@img/sharp-${os}-${cpu}`],
            contents: "dependencies",
            place: "toolchain",
        },
        {
            chain: [view, "sharp", `@img/sharp-${os}-${cpu}`, `@img/sharp-libvips-${os}-${cpu}`],
            contents: "dependencies",
            place: "toolchain",
        },

        // the runtime declarations
        { chain: [build, "@types/bun"], contents: "dependencies", place: "toolchain" },
        {
            chain: [build, "@cloudflare/workers-types"],
            contents: "dependencies",
            place: "toolchain",
        },

        // the packages builds name
        { chain: [build, "@destack/test"], contents: "metadata", place: "toolchain" },
        { chain: [build, "@destack/package"], contents: "metadata", place: "toolchain" },
        { chain: [build, "@destack/resource"], contents: "metadata", place: "toolchain" },

        // the linter, the formatter and the type-aware linter
        { chain: [check, "oxlint"], contents: "dependencies", place: "toolchain" },
        {
            chain: [check, "oxlint", `@oxlint/binding-${native}`],
            contents: "dependencies",
            place: "toolchain",
        },
        { chain: [check, "oxfmt"], contents: "dependencies", place: "toolchain" },
        {
            chain: [check, "oxfmt", `@oxfmt/binding-${native}`],
            contents: "dependencies",
            place: "toolchain",
        },
        {
            chain: [check, "oxlint-tsgolint", `@oxlint-tsgolint/${os}-${cpu}`],
            contents: "dependencies",
            place: "toolchain",
        },
    ];
}

/** The node_modules directories a distribution writes, laid out as npm installs packages. */
class NodeModules {
    /** The installation each hoisted name has, across every place. */
    readonly #hoisted = new Map<string, string>();
    /** The installation each nested directory has. */
    readonly #nested = new Map<string, string>();
    /** The file system the directories live on. */
    readonly #files: FileSystem;

    /** Lay out node_modules directories on a file system. */
    constructor(files: FileSystem) {
        this.#files = files;
    }

    /** Copy a package and its dependencies, hoisted, else nested under the dependent. */
    async copy(
        name: string,
        directory: string,
        modules: string,
        ancestors: readonly string[] = [],
    ): Promise<void> {
        // find the installation Node resolves from the dependent: the nearest nested one, else the hoisted one
        const files = this.#files;
        const real = await files.realPath(directory);
        const nearest = ancestors
            .toReversed()
            .map((each) => this.#nested.get(join(each, name)))
            .find((each) => each !== undefined);
        const resolved = nearest ?? this.#hoisted.get(name);
        const parent = ancestors.at(-1);

        // keep an installation the dependent already resolves
        if (resolved === real) {
            return;
        }
        // hoist a package no place has yet
        else if (resolved === undefined) {
            this.#hoisted.set(name, real);
        }
        // nest another version under its dependent, as the root has one
        else if (parent !== undefined && !this.#nested.has(join(parent, name))) {
            this.#nested.set(join(parent, name), real);
        }
        // refuse a second version where nothing can nest it
        else {
            throw new Error(
                `the toolchain needs two installations of '${name}': '${resolved}' and '${real}'`,
            );
        }

        // copy the package's own files, leaving out its installed dependencies
        const destination =
            resolved === undefined ? join(modules, name) : join(parent ?? modules, name);
        await copyTree(real, destination, files);

        // copy its dependencies, resolved from the package, as seen from its own node_modules
        const manifest = PackageManifest.parse(
            JSON.parse(await files.readTextFile(join(real, "package.json"))),
        );
        const below = [...ancestors, join(destination, "node_modules")];
        for (const dependency of Object.keys(manifest.dependencies ?? {})) {
            const located = await locate(dependency, real, files);
            await this.copy(dependency, located, modules, below);
        }
    }
}

/** Copy a directory's files with their links followed, leaving out installed dependencies. */
async function copyTree(source: string, destination: string, files: FileSystem): Promise<void> {
    // create the directory, and list its entries but node_modules
    await files.mkdir(destination, { recursive: true });
    const entries = await files.readDirectory(source);
    const copied = entries.filter((entry) => basename(entry.name) !== "node_modules");

    // copy each entry, a link as what it points at
    for (const entry of copied) {
        const from = join(source, entry.name);
        const to = join(destination, entry.name);
        const kind = entry.kind === "symlink" ? (await files.stat(from)).kind : entry.kind;

        // copy a directory's entries
        if (kind === "directory") {
            await copyTree(from, to, files);
        }
        // copy a file's bytes
        else {
            await files.copyFile(from, to);
        }
    }
}

/** Find an installed package's directory from a package directory. */
async function locate(name: string, from: string, files: FileSystem): Promise<string> {
    return files.realPath(
        dirname(createRequire(join(from, "package.json")).resolve(`${name}/package.json`)),
    );
}

/** Read the installed package containing a module: its name, directory and module path. */
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
async function isModulePackage(directory: string, files: FileSystem): Promise<boolean> {
    const manifest = PackageManifest.parse(
        JSON.parse(await files.readTextFile(join(directory, "package.json"))),
    );

    return manifest.type === "module";
}

/** Replace each `import.meta.url` of a module with another URL expression. */
function replaceModuleUrl(path: string, source: string, url: string): string {
    // find each import.meta.url
    const parsed = parseSync(path, source);
    if (parsed.errors.length > 0) {
        throw new Error(`invalid toolchain module: '${path}'`);
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
