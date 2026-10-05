import { graph, type Package } from "@destack/package";
import type { SourceRange } from "@destack/package/source";
import type { TestDeclaration } from "@destack/test/inspect";

/** The tests a build declares with the package defining their kinds. */
export interface TestSource {
    /** The package defining the test and suite kinds. */
    readonly package: Package;
    /** The tests and suites of every module. */
    readonly declarations: readonly TestDeclaration[];
}

/** A test or suite of a module as a declaration with its source range. */
export interface DescribedTest {
    /** The test's declaration. */
    readonly declaration: graph.Declaration;
    /** The range of the registering call. */
    readonly source: SourceRange;
}

/** The tests and suites of each module, as declarations covering what they call. */
export class TestGraph {
    /** The tests of each module, by module path. */
    readonly #tests = new Map<string, DescribedTest[]>();

    /** Describe each test at the symbol its suite titles and title name. */
    constructor(source: Package, tests: TestSource) {
        for (const test of tests.declarations) {
            const symbol = graph.Moniker.of({
                packageId: source.id,
                module: test.file,
                name: [...test.suites, test.name].join(" › "),
            });
            const described: DescribedTest = {
                declaration: {
                    moniker: `${symbol}:${test.kind}`,
                    symbol,
                    kind: test.kind,
                    package: tests.package.id,
                    name: [...test.suites, test.name].join(" › "),
                    description: { modifiers: test.modifiers },
                },
                source: { file: test.file, start: test.start, end: test.end },
            };
            this.#tests.set(test.file, [...(this.#tests.get(test.file) ?? []), described]);
        }
    }

    /** List a module's tests in source order. */
    describe(path: string): readonly DescribedTest[] {
        return this.#tests.get(path) ?? [];
    }

    /** Report whether a module declares tests. */
    has(path: string): boolean {
        return this.#tests.has(path);
    }
}
