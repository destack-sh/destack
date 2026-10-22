import { cp, mkdir, mkdtemp, realpath, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { type DependencyResolution, PackageJson } from "@destack/package";
import { type PackageBuildOptions, PackageBuild } from "../build/build.ts";
import { DependencyTree } from "../dependency/tree.ts";
import { Workspace, type WorkspaceCompilers } from "../dependency/workspace.ts";
import { BuildError } from "../error/index.ts";
import { readLockfile, readPackageJson } from "../source/index.ts";
import { LocalPackageStore } from "../store/local.ts";

/** A package copied for one test, built on its workspace closure's stored units. */
export class PackageCopy implements AsyncDisposable {
    /** The temporary directory with the copy, its builds and their trees. */
    readonly directory: string;
    /** The copied package. */
    readonly source: string;
    /** The builds of the units it depends on, as its build takes them. */
    readonly dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The store of the units' builds. */
    readonly store: LocalPackageStore;
    /** The compilers building the copy into the store. */
    readonly #compilers: WorkspaceCompilers;

    /** Store a copied package, its dependencies' builds and the compilers building it. */
    private constructor(
        directory: string,
        source: string,
        dependencies: Readonly<Record<string, DependencyResolution>>,
        compilers: WorkspaceCompilers,
    ) {
        // store the copy, its dependencies and the compilers with their store
        this.directory = directory;
        this.source = source;
        this.dependencies = dependencies;
        this.store = compilers.store;
        this.#compilers = compilers;
    }

    /** Copy a package beside its workspace's lockfile, linking its members, and store its units. */
    static async open(fixture: URL, compilers: WorkspaceCompilers): Promise<PackageCopy> {
        // copy the package without its installed tree, beside a link to its workspace's lockfile
        const origin = fileURLToPath(fixture);
        const lockfile = await readLockfile(origin);
        const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-test-source-")));
        const source = join(directory, "source");
        await cp(origin, source, {
            recursive: true,
            filter: (path) => !path.split(/[/\\]/u).includes("node_modules"),
        });
        await symlink(join(lockfile.directory, "bun.lock"), join(directory, "bun.lock"));

        // link each dependency it declares to its workspace member, as an install does
        const members = new Map(
            Object.entries(lockfile.workspaces).flatMap(([path, member]) =>
                member.name === undefined ? [] : [[member.name, join(lockfile.directory, path)]],
            ),
        );
        const packageJson = await readPackageJson(source);
        const declared = [
            ...PackageJson.runtimeDependencies(packageJson),
            ...Object.keys(packageJson.devDependencies),
        ];
        for (const name of new Set(declared)) {
            const member = members.get(name);
            if (member === undefined) {
                throw new BuildError("BUILD_FAILED", `the workspace has no member ${name}`);
            }
            const link = join(source, "node_modules", name);
            await mkdir(dirname(link), { recursive: true });
            await symlink(member, link, "junction");
        }

        // store the units it depends on
        const workspace = await Workspace.open(source, compilers);
        const dependencies = await workspace.storeDependencies({});

        return new PackageCopy(directory, source, dependencies, compilers);
    }

    /** Build the copy on its dependencies' builds with its warm compiler. */
    async build(
        request: Omit<PackageBuildOptions, "directory" | "dependencies">,
    ): Promise<PackageBuild> {
        const options = { ...request, directory: this.source, dependencies: this.dependencies };

        return await this.#compilers.use(this.source, (compiler) => compiler.build(options));
    }

    /** Write a build into a directory with the builds its outputs import placed beside it. */
    async deploy(build: PackageBuild, destination: string): Promise<void> {
        // write the build, then place the builds its outputs record and link them into it
        await PackageBuild.write(build.reader, destination);
        const recorded = Object.values(build.manifest.outputs).flatMap((output) =>
            Object.values(output.dependencies),
        );
        const unique = [...new Map(recorded.map((each) => [each.manifest, each])).values()];
        const tree = await DependencyTree.materialize(
            this.store,
            unique,
            join(this.directory, "tree"),
        );
        await tree.link(join(destination, "node_modules"));
    }

    /** Remove the copy, its builds and their trees. */
    async [Symbol.asyncDispose](): Promise<void> {
        await rm(this.directory, { recursive: true });
    }
}
