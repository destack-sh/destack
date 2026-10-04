import { readFile, stat } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { schema } from "@destack/schema";

/** The workspace members of a Bun lockfile by directory, with the root as "". */
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
    /** The package directory relative to the workspace root with the root as ".". */
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

    /** Read the workspace enclosing a directory, absent without an enclosing lockfile. */
    static async read(directory: string): Promise<Workspace | undefined> {
        // read the lockfile of the enclosing workspace
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
        const lockfile = WorkspaceLockfile.parse(JSON.parse(dropTrailingCommas(text)));
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

    /** List the members changed files affect, absent when a file lies outside every member. */
    affected(files: readonly string[]): string[] | undefined {
        // collect the members enclosing each changed file
        const changed = new Set<string>();
        for (const file of files) {
            const holders = this.members.filter(
                (member) => member.directory !== "." && file.startsWith(`${member.directory}/`),
            );
            // affect every member through a file outside them other than Markdown
            if (holders.length === 0 && !file.endsWith(".md")) {
                return undefined;
            }
            // mark nested members and their enclosing members alike
            for (const member of holders) {
                changed.add(member.directory);
            }
        }

        // add the dependents of each changed member without the root
        const affected = [...changed].flatMap((directory) => this.dependents(directory));

        return [...new Set(affected)].filter((directory) => directory !== ".").toSorted();
    }

    /** Walk the members one leads to along an edge, the member included. */
    #closure(directory: string, next: (member: WorkspaceMember) => readonly string[]): string[] {
        // walk depth first from the member and visit each member once
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

/** Find the nearest directory holding a Bun lockfile, the directory itself without one. */
export async function locateWorkspace(directory: string): Promise<string> {
    // accept the nearest directory holding the lockfile
    for (let current = resolve(directory); ; current = dirname(current)) {
        if (await exists(join(current, "bun.lock"))) {
            return current;
        }
        if (dirname(current) === current) {
            return resolve(directory);
        }
    }
}

/** Report whether a path exists. */
async function exists(path: string): Promise<boolean> {
    try {
        await stat(path);

        return true;
    } catch (error) {
        if (!isMissing(error)) {
            throw error;
        }

        return false;
    }
}

/** Report whether a file system error names a missing path. */
function isMissing(error: unknown): boolean {
    return error instanceof Error && "code" in error && error.code === "ENOENT";
}

/** Drop the trailing commas Bun writes into lockfiles. */
function dropTrailingCommas(text: string): string {
    // copy the text character by character and track open strings
    let result = "";
    let isString = false;
    for (let index = 0; index < text.length; index++) {
        const character = text.charAt(index);
        // copy strings whole with their escapes
        if (isString) {
            result += character;
            if (character === "\\") {
                result += text.charAt(++index);
            } else if (character === '"') {
                isString = false;
            }
        }
        // skip a comma whose next significant character closes an object or array
        else if (character === "," && isClosing(text, index + 1)) {
            continue;
        }
        // copy everything else
        else {
            isString = character === '"';
            result += character;
        }
    }

    return result;
}

/** Report whether the next character after whitespace closes an object or array. */
function isClosing(text: string, start: number): boolean {
    // skip whitespace before the next character
    let index = start;
    while (index < text.length && /\s/u.test(text.charAt(index))) {
        index++;
    }

    return text.charAt(index) === "}" || text.charAt(index) === "]";
}
