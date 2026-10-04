import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { JSONC } from "bun";
import { schema } from "@destack/schema";
import { locateWorkspace } from "./dependency.ts";
import { isMissing } from "../error/index.ts";

/** The workspace members a Bun lockfile records, by directory relative to the workspace root, the root as "". */
const WorkspaceLockfile = schema
    .object({
        workspaces: schema.record(
            schema.string(),
            schema
                .object({
                    name: schema.string().exactOptional(),
                    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
                    devDependencies: schema
                        .record(schema.string(), schema.string())
                        .exactOptional(),
                    peerDependencies: schema
                        .record(schema.string(), schema.string())
                        .exactOptional(),
                    optionalDependencies: schema
                        .record(schema.string(), schema.string())
                        .exactOptional(),
                })
                .strip(),
        ),
    })
    .strip();

/** One package of a workspace. */
export interface WorkspaceMember {
    /** The package directory relative to the workspace root, with forward slashes, the root as ".". */
    readonly directory: string;
    /** The package name. */
    readonly name: string;
    /** The directories of the members it depends on through any dependency group. */
    readonly dependencies: readonly string[];
}

/** A Bun workspace: its root and its member packages, as its lockfile records them. */
export class Workspace {
    /** The absolute directory holding the lockfile. */
    readonly root: string;
    /** The member packages, the root included when it is named. */
    readonly members: readonly WorkspaceMember[];

    /** Keep a workspace's members. */
    private constructor(root: string, members: readonly WorkspaceMember[]) {
        this.root = root;
        this.members = members;
    }

    /** Read the workspace enclosing a directory from its lockfile, absent when no lockfile encloses it. */
    static async read(directory: string): Promise<Workspace | undefined> {
        // read the lockfile, absent for a directory outside any workspace
        const root = await locateWorkspace(directory);
        const text = await readFile(join(root, "bun.lock"), "utf8").catch((error: unknown) => {
            if (!isMissing(error)) {
                throw error;
            }
        });
        if (text === undefined) {
            return undefined;
        }

        // read the members the lockfile records
        const lockfile = WorkspaceLockfile.parse(JSONC.parse(text));
        const named = Object.entries(lockfile.workspaces).flatMap(([path, member]) => {
            const location = path === "" ? "." : path;

            return member.name === undefined ? [] : [{ path: location, name: member.name, member }];
        });

        // link each member to the members its workspace dependencies name
        const directories = new Map(named.map(({ path, name }) => [name, path]));
        const members = named.map(({ path, name, member }) => {
            const specifiers = {
                ...member.optionalDependencies,
                ...member.peerDependencies,
                ...member.devDependencies,
                ...member.dependencies,
            };
            const dependencies = Object.entries(specifiers).flatMap(([dependent, specifier]) => {
                const dependency = directories.get(dependent);

                return specifier.startsWith("workspace:") && dependency !== undefined
                    ? [dependency]
                    : [];
            });

            return { directory: path, name, dependencies };
        });

        return new Workspace(root, members);
    }

    /** List a member and every member it depends on, directly or through others. */
    dependencies(directory: string): string[] {
        return this.#closure(directory, (member) => member.dependencies);
    }

    /** List a member and every member depending on it, directly or through others. */
    dependents(directory: string): string[] {
        return this.#closure(directory, (member) =>
            this.members
                .filter((other) => other.dependencies.includes(member.directory))
                .map((other) => other.directory),
        );
    }

    /** Walk the members one leads to along an edge, the member included. */
    #closure(directory: string, next: (member: WorkspaceMember) => readonly string[]): string[] {
        // walk depth first from the member, each member once
        const members = new Map(this.members.map((member) => [member.directory, member]));
        const visited = new Set<string>();
        const pending = [directory];
        for (let current = pending.pop(); current !== undefined; current = pending.pop()) {
            const member = members.get(current);
            if (member === undefined || visited.has(current)) {
                continue;
            }
            visited.add(current);
            pending.push(...next(member));
        }

        return [...visited];
    }
}
