import type { FileSystem } from "@destack/environment";
import { relative, resolve, sep } from "node:path";
import { Definition, type Expectation } from "@destack/package";
import type { Diagnostic } from "../diagnostic/index.ts";
import { readManifest } from "./manifest.ts";

/** An expectation and the directory of the destack.json that declares it. */
export interface DeclaredExpectation {
    /** The directory its files are relative to. */
    readonly directory: string;
    /** The accepted findings. */
    readonly expectation: Expectation;
}

/** Read a package's expectations, and every member's when the directory is a workspace root. */
export async function readExpectations(
    directory: string,
    files: FileSystem,
): Promise<DeclaredExpectation[]> {
    // read the declarations of the directory and its members
    const members = await workspaceMembers(directory, files);
    const expectations = await Promise.all(
        [directory, ...members].map((member) => packageExpectations(member, files)),
    );

    return expectations.flat();
}

/** Write expectations as rule overrides for their files, relative to a configuration directory. */
export function expectationOverrides(
    expectations: readonly DeclaredExpectation[],
    directory: string,
): { files: string[]; rules: Record<string, "off"> }[] {
    return expectations.map((declared) => ({
        files: filesOf(declared).map((file) => relative(directory, file).split(sep).join("/")),
        rules: Object.fromEntries(declared.expectation.rules.map((rule) => [rule, "off" as const])),
    }));
}

/** Drop the findings expectations accept. */
export function applyExpectations(
    diagnostics: readonly Diagnostic[],
    expectations: readonly DeclaredExpectation[],
    directory: string,
    selection: readonly string[],
): Diagnostic[] {
    // return the findings no expectation accepts, noting the expectations that matched
    const matched = new Set<DeclaredExpectation>();
    const unexpected = diagnostics.filter((diagnostic) => {
        // find the expectation for the finding's file and rule
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
        ...unexpected,
        ...stale.map(({ expectation, ...declared }): Diagnostic => ({
            code: "destack(unfulfilled-expectation)",
            message: `nothing matches the expectation of ${expectation.rules.join(", ")} (${expectation.reason}); remove it`,
            severity: "error",
            filename: relative(directory, filesOf({ ...declared, expectation })[0] ?? directory),
            labels: [],
        })),
    ];
}

/** Read the expectations a destack.json declares for its package and workspace. */
async function packageExpectations(
    directory: string,
    files: FileSystem,
): Promise<DeclaredExpectation[]> {
    // read the package's and the workspace's declarations
    const path = resolve(directory, "destack.json");
    if (!(await files.exists(path))) {
        return [];
    }
    const definition = Definition.read(await files.readTextFile(path));
    const declared = [
        ...(Definition.package(definition)?.check?.expect ?? []),
        ...(definition.workspace?.check?.expect ?? []),
    ];

    return declared.map((expectation) => ({ directory, expectation }));
}

/** List a workspace root's member directories, none for a directory that is not a workspace. */
export async function workspaceMembers(directory: string, files: FileSystem): Promise<string[]> {
    // read the member patterns
    const workspaces = (await readManifest(directory, files))?.workspaces;

    // expand each pattern segment by segment, retaining directories with a package manifest
    const members: string[] = [];
    for (const pattern of workspaces ?? []) {
        let directories = [directory];
        for (const segment of pattern.split("/")) {
            const next: string[] = [];
            for (const parent of directories) {
                if (segment === "*") {
                    const entries = (await files.exists(parent))
                        ? await files.readDirectory(parent)
                        : [];
                    next.push(
                        ...entries
                            .filter((entry) => entry.kind === "directory")
                            .map((entry) => entry.name)
                            .toSorted()
                            .map((name) => resolve(parent, name)),
                    );
                } else {
                    next.push(resolve(parent, segment));
                }
            }
            directories = next;
        }
        for (const member of directories) {
            if (await files.exists(resolve(member, "package.json"))) {
                members.push(member);
            }
        }
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
