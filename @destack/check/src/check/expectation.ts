import { readdir, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { relative, resolve, sep } from "node:path";
import { Definition, type Expectation } from "@destack/package";
import type { Diagnostic } from "../inspect/index.ts";
import { readManifest } from "./manifest.ts";

/** An expectation and the directory of the destack.json declaring it, which its files are relative to. */
export interface DeclaredExpectation {
    /** The directory its files are relative to. */
    readonly directory: string;
    /** The accepted findings. */
    readonly expectation: Expectation;
}

/** Read the expectations of a package, and of every member when the directory is a workspace root. */
export async function readExpectations(directory: string): Promise<DeclaredExpectation[]> {
    // read the directory's own and its members' declarations
    const members = await workspaceMembers(directory);
    const expectations = await Promise.all([directory, ...members].map(packageExpectations));

    return expectations.flat();
}

/** Write expectations as rule overrides for the files they name, relative to a configuration's directory. */
export function expectationOverrides(
    expectations: readonly DeclaredExpectation[],
    directory: string,
): { files: string[]; rules: Record<string, "off"> }[] {
    return expectations.map((declared) => ({
        files: filesOf(declared).map((file) => relative(directory, file).split(sep).join("/")),
        rules: Object.fromEntries(declared.expectation.rules.map((rule) => [rule, "off" as const])),
    }));
}

/** Drop the findings expectations accept, and report each expectation over checked files that nothing matched. */
export function applyExpectations(
    diagnostics: readonly Diagnostic[],
    expectations: readonly DeclaredExpectation[],
    directory: string,
    selection: readonly string[],
): Diagnostic[] {
    // keep the findings no expectation accepts, noting the expectations that matched
    const matched = new Set<DeclaredExpectation>();
    const kept = diagnostics.filter((diagnostic) => {
        // find the expectation naming the finding's file and rule
        const file = resolve(directory, diagnostic.filename);
        const expectation = expectations.find(
            (candidate) =>
                filesOf(candidate).includes(file) &&
                candidate.expectation.rules.some((rule) => codeOf(rule) === diagnostic.code),
        );
        if (expectation !== undefined) {
            matched.add(expectation);
        }

        return expectation === undefined;
    });

    // refuse expectations over checked files that no finding met
    const selected = selection.map((entry) => resolve(directory, entry));
    const stale = expectations.filter(
        (declared) =>
            !matched.has(declared) &&
            filesOf(declared).every((file) =>
                selected.some((root) => file === root || file.startsWith(`${root}${sep}`)),
            ),
    );

    return [
        ...kept,
        ...stale.map(({ expectation, ...declared }): Diagnostic => ({
            code: "destack(unfulfilled-expectation)",
            message: `nothing matches the expectation of ${expectation.rules.join(", ")} (${expectation.reason}); remove it`,
            severity: "error",
            filename: relative(directory, filesOf({ ...declared, expectation })[0] ?? directory),
            labels: [],
        })),
    ];
}

/** Read the expectations a destack.json declares for its package and workspace, absent without one. */
async function packageExpectations(directory: string): Promise<DeclaredExpectation[]> {
    // read the package's and the workspace's declarations
    const path = resolve(directory, "destack.json");
    if (!existsSync(path)) {
        return [];
    }
    const definition = Definition.read(await readFile(path, "utf8"));
    const declared = [
        ...(Definition.package(definition)?.check?.expect ?? []),
        ...(definition.workspace?.check?.expect ?? []),
    ];

    return declared.map((expectation) => ({ directory, expectation }));
}

/** List a workspace root's member directories, none for a directory that is no workspace root. */
async function workspaceMembers(directory: string): Promise<string[]> {
    // read the member patterns
    const workspaces = (await readManifest(directory))?.workspaces;

    // expand each pattern segment by segment, keeping directories with a package manifest
    const members: string[] = [];
    for (const pattern of workspaces ?? []) {
        let directories = [directory];
        for (const segment of pattern.split("/")) {
            const next: string[] = [];
            for (const parent of directories) {
                if (segment === "*") {
                    const entries = existsSync(parent)
                        ? await readdir(parent, { withFileTypes: true })
                        : [];
                    next.push(
                        ...entries
                            .filter((entry) => entry.isDirectory())
                            .map((entry) => resolve(parent, entry.name)),
                    );
                } else {
                    next.push(resolve(parent, segment));
                }
            }
            directories = next;
        }
        members.push(
            ...directories.filter((member) => existsSync(resolve(member, "package.json"))),
        );
    }

    return members;
}

/** Resolve an expectation's files to absolute paths. */
function filesOf(declared: DeclaredExpectation): string[] {
    return declared.expectation.files.map((file) => resolve(declared.directory, file));
}

/** Name a rule as the linter's diagnostics code it: `eslint/no-console` as `eslint(no-console)`. */
function codeOf(rule: string): string {
    const [namespace, name] = rule.split("/");

    return `${namespace}(${name})`;
}
