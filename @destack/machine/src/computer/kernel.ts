import type { Environment } from "@destack/environment";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** The file beside a runtime's shipped builds listing the packages the runtime provides. */
export const KERNEL_FILE = "kernel.json";

/** The workspace scopes a kernel walk enters, leaving npm packages to the builds bundling them. */
const WORKSPACE_SCOPES = ["@destack/", "@platform/", "@app/"] as const;

/** The subpaths a runtime imports another package through without providing it. */
const EXEMPT_SUBPATHS: ReadonlySet<string> = new Set(["declare", "client"]);

/** The loader Bun's transpiler reads a module with, by file extension. */
const LOADERS: Readonly<Record<string, "ts" | "tsx" | "js">> = {
    ts: "ts",
    tsx: "tsx",
    js: "js",
    mjs: "js",
};

/** A provided package's id, by its name, as the kernel file keeps it. */
const KernelPackages = schema.record(schema.string().min(1), PackageId);

/** The fields of a package's manifests a kernel reads. */
const PackageFiles = {
    /** A package's name. */
    name: schema.looseObject({ name: schema.string().min(1) }),
    /** A package's id. */
    id: schema.looseObject({ id: PackageId }),
};

/** The packages a runtime provides to the servers it loads, as Node provides its built-ins. */
export class Kernel {
    /** The provided packages' ids, by name. */
    readonly packages: ReadonlyMap<string, PackageId>;

    /** Take the provided packages. */
    constructor(packages: ReadonlyMap<string, PackageId>) {
        this.packages = packages;
    }

    /** Walk a runtime entry's imports as Bun resolves them, but other packages' exempt ones. */
    static async read(entry: string, environment: Environment<"files">): Promise<Kernel> {
        // follow each module once, reading the package of each directory once
        const { files } = environment;
        const packages = new Map<string, PackageId>();
        const directories = new Map<string, string>();
        const visited = new Set<string>();
        const pending = [entry];
        for (let file = pending.pop(); file !== undefined; file = pending.pop()) {
            // read the module's package, provided once the runtime loads one of its modules
            if (visited.has(file)) {
                continue;
            }
            visited.add(file);
            const directory = Kernel.#directory(file);
            const name = await Kernel.#package(directory, packages, directories, files);

            // follow its relative imports and its workspace packages' modules, but other packages' declarations and clients
            const loader = LOADERS[file.slice(file.lastIndexOf(".") + 1)];
            if (loader === undefined) {
                continue;
            }
            const text = (await files.readTextFile(file)).replace(/^#!.*/u, "");
            for (const { path: specifier } of new Bun.Transpiler({ loader }).scanImports(text)) {
                if (Kernel.#isFollowed(specifier, name)) {
                    pending.push(Bun.resolveSync(specifier, file.slice(0, file.lastIndexOf("/"))));
                }
            }
        }

        return new Kernel(packages);
    }

    /** Read the kernel a launcher wrote beside a runtime's shipped builds. */
    static async open(directory: string, environment: Environment<"files">): Promise<Kernel> {
        const text = await environment.files.readTextFile(`${directory}/${KERNEL_FILE}`);

        return Kernel.parse(text);
    }

    /** Read a kernel from its file's text, as a binding carries it. */
    static parse(text: string): Kernel {
        return new Kernel(new Map(Object.entries(KernelPackages.parse(JSON.parse(text)))));
    }

    /** Write the kernel beside a runtime's shipped builds. */
    async write(directory: string, environment: Environment<"files">): Promise<void> {
        await environment.files.writeTextFile(`${directory}/${KERNEL_FILE}`, this.text());
    }

    /** Write the kernel's file text: each package's id by its name, in name order. */
    text(): string {
        const sorted = [...this.packages].toSorted(([left], [right]) => (left < right ? -1 : 1));

        return `${JSON.stringify(Object.fromEntries(sorted), null, 4)}\n`;
    }

    /** Report whether a specifier names a module of a provided package. */
    provides(specifier: string): boolean {
        return this.packages.has(Kernel.#name(specifier));
    }

    /** Report whether a walk follows a specifier: relative, or workspace but exempt. */
    static #isFollowed(specifier: string, importer: string): boolean {
        // follow a relative module, and leave npm and runtime modules to the builds bundling them
        if (specifier.startsWith(".")) {
            return true;
        } else if (!WORKSPACE_SCOPES.some((scope) => specifier.startsWith(scope))) {
            return false;
        }

        // leave another package's declarations and client
        const name = Kernel.#name(specifier);
        const [subpath = ""] = specifier.slice(name.length + 1).split("/");

        return name === importer || !EXEMPT_SUBPATHS.has(subpath);
    }

    /** Read a scoped specifier's package name, such as @destack/db for @destack/db/test. */
    static #name(specifier: string): string {
        return specifier.split("/").slice(0, 2).join("/");
    }

    /** Find the workspace package directory a module lies in, by its scope's segment. */
    static #directory(file: string): string {
        // take the last scope segment and the package directory after it
        const at = Math.max(
            ...WORKSPACE_SCOPES.map((scope) => file.lastIndexOf(`/${scope.slice(0, -1)}/`)),
        );
        if (at === -1) {
            throw new TypeError(`a runtime module lies in no workspace package: ${file}`);
        }
        const end = file.indexOf("/", file.indexOf("/", at + 1) + 1);

        return file.slice(0, end);
    }

    /** Read the name of a package directory, recording its id once among the provided packages. */
    static async #package(
        directory: string,
        packages: Map<string, PackageId>,
        directories: Map<string, string>,
        files: Environment<"files">["files"],
    ): Promise<string> {
        // read a directory's package once
        const known = directories.get(directory);
        if (known !== undefined) {
            return known;
        }
        const { name } = PackageFiles.name.parse(
            JSON.parse(await files.readTextFile(`${directory}/package.json`)),
        );
        const { id } = PackageFiles.id.parse(
            JSON.parse(await files.readTextFile(`${directory}/destack.json`)),
        );
        directories.set(directory, name);
        packages.set(name, id);

        return name;
    }
}
