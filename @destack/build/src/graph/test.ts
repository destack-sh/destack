import { graph, type PackageVersion } from "@destack/package";
import type { TestDeclaration } from "../outline/test.ts";
import type { Registration } from "./example.ts";

/** The tests a build declares with the package defining their kinds. */
export interface TestSource {
    /** The package defining the test and suite kinds. */
    readonly package: PackageVersion;
    /** The tests and suites of every module. */
    readonly declarations: readonly TestDeclaration[];
}

/** The tests and suites of each module, as declarations covering what they call. */
export class TestGraph {
    /** The tests of each module, by module path. */
    readonly #tests = new Map<string, Registration[]>();

    /** Describe each test as a declaration at its module, named by its suite titles and title. */
    constructor(source: PackageVersion, tests: TestSource) {
        for (const test of tests.declarations) {
            const module = graph.Moniker.of({ packageId: source.id, module: test.file });
            const registered: Registration = {
                declaration: graph.Declaration.at(module, {
                    kind: test.kind,
                    kindPackageId: tests.package.id,
                    name: [...test.suites, test.name].join(" › "),
                    description: { modifiers: test.modifiers },
                }),
                edges: [],
                source: { file: test.file, start: test.start, end: test.end },
            };
            this.#tests.set(test.file, [...(this.#tests.get(test.file) ?? []), registered]);
        }
    }

    /** List a module's tests in source order. */
    describe(path: string): readonly Registration[] {
        return this.#tests.get(path) ?? [];
    }

    /** Report whether a module declares tests. */
    has(path: string): boolean {
        return this.#tests.has(path);
    }
}
