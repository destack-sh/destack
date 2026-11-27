import { join, relative } from "node:path";
import { format } from "oxfmt";
import { GitLease } from "@destack/git/local";
import { repository } from "@destack/registry/object";
import { Lease } from "@destack/resource";
import { Workspace } from "@destack/build";
import { Commit, schema, Uuid } from "@destack/schema";
import type { EditInput, EditOutput } from "@destack/stack/object";
import { PackageSource } from "@destack/stack/source";
import { MachineError } from "../error/index.ts";
import { WorkingTree } from "./checkout/index.ts";
import type { RunnerContext } from "./runner.ts";

/** The pushes an edit tries before failing: three, as a branch rarely moves during a 2 s edit. */
const PUSH_ATTEMPTS = 3;

/** The formatter configuration a repository keeps at its root, as oxfmt reads it. */
const FORMAT_CONFIGURATION = ".oxfmtrc.json";

/** The formatter options an edit writes with, those of a repository's configuration. */
const FormatOptions = schema.looseObject({
    tabWidth: schema.number().int().positive().exactOptional(),
    printWidth: schema.number().int().positive().exactOptional(),
    useTabs: schema.boolean().exactOptional(),
    semi: schema.boolean().exactOptional(),
    singleQuote: schema.boolean().exactOptional(),
    trailingComma: schema.enum(["all", "es5", "none"]).exactOptional(),
    endOfLine: schema.enum(["lf", "crlf", "cr"]).exactOptional(),
});

/** An edit a runner does: one checked commit on its branch's head, again on a head that moved. */
export class Edit {
    /** The edit, its author and its repository. */
    readonly #input: EditInput;
    /** What the runner runs it with. */
    readonly #context: RunnerContext;

    /** Take an edit and what the runner runs it with. */
    constructor(input: EditInput, context: RunnerContext) {
        this.#input = input;
        this.#context = context;
    }

    /** Push the edit, again while other pushes move its branch meanwhile. */
    async push(): Promise<EditOutput> {
        // lease the repository to write, as the run's actor through the space
        const target = repository.target(this.#input.source.repository);
        const lease = Lease.parse(await this.#context.call(repository.call("push", target)));

        // push on the branch's head until a push lands, then record the branch's commit, which followers read, as a lease push reaches no server
        for (let attempt = 0; attempt < PUSH_ATTEMPTS; attempt += 1) {
            const pushed = await this.#pushOnce(lease);
            if (pushed !== undefined) {
                if (pushed.commit !== pushed.base) {
                    await this.#context.call(repository.call("refresh", target));
                }

                return pushed;
            }
        }

        // refuse an edit whose branch kept moving
        throw MachineError.of(
            "EDIT_BLOCKED",
            `cannot push the edit: its branch moved ${PUSH_ATTEMPTS} times while it ran`,
        );
    }

    /** Commit the edit on its branch's head once, answering nothing when another push moved it. */
    async #pushOnce(lease: Lease): Promise<EditOutput | undefined> {
        // fetch the branch's head into a new working tree
        const { source, author, message } = this.#input;
        const { environment, signal } = this.#context;
        const root = join(this.#context.directory, Uuid.v4(environment.random));
        try {
            const base = Commit.parse(
                await GitLease.branch(lease, source.branch, root, environment, signal),
            );
            const directory = WorkingTree.package(root, source.directory).path;

            // apply the edit, completing at the base when it changes nothing, as adding a present dependency does
            const paths = await this.#apply({ root, directory });
            if (paths.length === 0) {
                return { base, commit: base };
            }

            // compile the package
            await this.#context.builder.compile(directory, signal);

            // push the commit while the branch is still at its base, else find whether it moved
            const commit = { message, author, paths };
            const branch = { branch: source.branch, base };
            try {
                const pushed = await GitLease.push(
                    lease,
                    root,
                    branch,
                    commit,
                    environment,
                    signal,
                );

                return { base, commit: Commit.parse(pushed) };
            } catch (error) {
                const head = await GitLease.head(lease, source.branch, root, environment, signal);
                if (head === base) {
                    throw error;
                }

                return undefined;
            }
        } finally {
            await environment.files.remove(root, { recursive: true, force: true });
        }
    }

    /** Apply the edit's operations and lock its dependencies, answering the changed paths. */
    async #apply(tree: {
        readonly root: string;
        readonly directory: string;
    }): Promise<readonly string[]> {
        // read the working tree's formatter options
        const { environment, signal } = this.#context;
        const { files, scheduler } = environment;
        const configuration = join(tree.root, FORMAT_CONFIGURATION);
        const options = (await files.exists(configuration))
            ? FormatOptions.parse(JSON.parse(await files.readTextFile(configuration)))
            : {};

        // apply each operation, then write the changed files
        const source = new PackageSource(
            (path) => files.readTextFile(join(tree.directory, path)),
            async (path, text) => (await scheduler.waitUntil(format(path, text, options))).code,
        );
        const { operations } = this.#input;
        for (const operation of operations) {
            await source.apply(operation);
        }
        const written = [...(await source.changed())].map(([path, text]) => ({
            path: join(tree.directory, path),
            text,
        }));
        const changed: typeof written = [];
        for (const { path, text } of written) {
            // write a file whose text differs, leaving one the edit restates
            if ((await files.readTextFile(path)) !== text) {
                await files.writeTextFile(path, text);
                changed.push({ path, text });
            }
        }
        if (changed.length === 0) {
            return [];
        }

        // lock and install changed dependencies, as `bun add` does
        const isDepending = operations.some((operation) =>
            operation.kind.startsWith("dependency."),
        );
        const lockfile = isDepending
            ? await Workspace.install(tree.directory, environment, { isFrozen: false, signal })
            : undefined;

        // answer the changed paths within the tree
        const paths = [
            ...changed.map(({ path }) => path),
            ...(lockfile === undefined ? [] : [lockfile]),
        ];

        return paths.map((path) => relative(tree.root, path));
    }
}
