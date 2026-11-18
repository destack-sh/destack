import { Text } from "@destack/schema";
import { graph, type PackageVersion } from "@destack/package";
import type { SourceRange } from "@destack/package/source";
import type { Outline } from "../outline/outline.ts";
import { BuildError } from "../error/index.ts";
import { DeclarationGraph } from "./declaration.ts";
import { TestGraph, type TestSource } from "./test.ts";
import { type ExampleDeclaration, ExampleGraph } from "./example.ts";
import { type ScenarioDeclaration, ScenarioGraph } from "./scenario.ts";

/** The examples and scenarios a build declares with the package defining their kinds. */
export interface ExampleSource {
    /** The package defining the example and scenario kinds. */
    readonly package: PackageVersion;
    /** The examples of every module. */
    readonly examples: readonly ExampleDeclaration[];
    /** The scenarios of every module. */
    readonly scenarios: readonly ScenarioDeclaration[];
}

/** The package-wide graph inputs each module's graph file draws on. */
interface PackageGraph {
    /** The package's declarations and their edges. */
    readonly declared: DeclarationGraph;
    /** The package's tests. */
    readonly tested: TestGraph;
    /** The package's examples. */
    readonly examples: ExampleGraph;
    /** The package's scenarios. */
    readonly scenarios: ScenarioGraph;
}

/** The edges of one module, each once. */
class Edges {
    /** The edges by their serialization. */
    readonly #edges = new Map<string, graph.Edge>();

    /** Add an edge, skipping an edge to its own source. */
    add(from: graph.Moniker, kind: graph.Edge["kind"], to: graph.Moniker): void {
        if (from !== to) {
            this.#edges.set(`${from} ${kind} ${to}`, { from, to, kind });
        }
    }

    /** List the edges by source, kind and target. */
    list(): graph.Edge[] {
        return [...this.#edges]
            .toSorted(([left], [right]) => Text.compare(left, right))
            .map(([, edge]) => edge);
    }
}

/** Describe each module's graph file: its declarations, its symbols and their edges. */
export function describeGraph(
    source: PackageVersion,
    outlines: readonly Outline[],
    declared: DeclarationGraph,
    tests: TestSource,
    played: ExampleSource,
): graph.Module[] {
    // refuse a declaration in a module the outlines leave out
    const paths = new Set(outlines.map((outline) => outline.path));
    const stray = declared
        .list()
        .find((declaration) => !paths.has(graph.Moniker.module(declaration.symbol)));
    if (stray !== undefined) {
        throw BuildError.of(
            "DECLARATION_INVALID",
            `declared '${stray.moniker}' is invalid: its module is no package module`,
        );
    }

    // collect the package's tests, examples and scenarios
    const tested = new TestGraph(source, tests);
    const examples = new ExampleGraph(source, played.package, played.examples, declared);
    const scenarios = new ScenarioGraph(source, played.package, played.scenarios, examples);

    return outlines.map((outline) =>
        describeModule(source, outline, { declared, tested, examples, scenarios }),
    );
}

/** Describe one module's exports, declarations, symbols and the edges between them. */
function describeModule(
    source: PackageVersion,
    outline: Outline,
    { declared, tested, examples, scenarios }: PackageGraph,
): graph.Module {
    // depend on each resolved import, and extend what each class and interface names
    const moniker = graph.Moniker.of({ packageId: source.id, module: outline.path });
    const edges = new Edges();
    for (const imported of outline.imports) {
        if (imported.target !== undefined) {
            edges.add(moniker, "depends", imported.target);
        }
    }
    for (const edge of outline.heritage) {
        edges.add(edge.from, edge.kind, edge.to);
    }

    // describe the module's declarations, tests, examples and scenarios with their edges
    const tests = tested.describe(outline.path);
    const moduleExamples = examples.describe(outline.path);
    const moduleScenarios = scenarios.describe(outline.path);
    const described = [
        ...declared.describe(outline.path),
        ...[...tests, ...moduleExamples, ...moduleScenarios].map((entry) => entry.declaration),
    ].toSorted((left, right) => Text.compare(left.moniker, right.moniker));
    for (const edge of [
        ...declared.edges(outline.path),
        ...[...moduleExamples, ...moduleScenarios].flatMap((entry) => entry.edges),
    ]) {
        edges.add(edge.from, edge.kind, edge.to);
    }

    // attribute each call and type reference to its innermost declaration or test, tests covering what they call
    const { symbols } = outline;
    const enclosings = [
        ...symbols.flatMap((symbol) =>
            symbol.declarations.map((declaration) => ({
                moniker: symbol.moniker,
                source: declaration.source,
            })),
        ),
        ...tests.map((test) => ({ moniker: test.declaration.moniker, source: test.source })),
    ];
    const covering = new Set(tests.map((test) => test.declaration.moniker));
    for (const use of outline.uses) {
        const from = enclose(enclosings, use.source) ?? moniker;
        edges.add(from, use.kind, use.to);
        if (covering.has(from) && isCovered(source, tested, use.to)) {
            edges.add(from, "covers", use.to);
            for (const declaration of declared.at(use.to)) {
                edges.add(from, "covers", declaration);
            }
        }
    }

    return graph.Module.parse(
        graph.Module.of({
            path: outline.path,
            digest: outline.digest,
            imports: outline.imports.map((imported) => imported.specifier),
            exports: outline.exports,
            declarations: described,
            symbols: symbols.toSorted((left, right) => Text.compare(left.moniker, right.moniker)),
            edges: edges.list(),
        }),
    );
}

/** Report whether a symbol is the package's, outside its test modules. */
function isCovered(source: PackageVersion, tested: TestGraph, symbol: graph.Moniker): boolean {
    // take the package's symbols, leaving those of the test modules themselves
    return (
        graph.Moniker.packageId(symbol) === source.id && !tested.has(graph.Moniker.module(symbol))
    );
}

/** Find the innermost symbol or test whose source range contains a range. */
function enclose(
    enclosings: readonly { readonly moniker: graph.Moniker; readonly source: SourceRange }[],
    range: SourceRange,
): graph.Moniker | undefined {
    let found: (typeof enclosings)[number] | undefined;
    for (const enclosing of enclosings) {
        const isInside =
            enclosing.source.file === range.file &&
            enclosing.source.start <= range.start &&
            range.end <= enclosing.source.end;
        const isNarrower =
            found === undefined ||
            enclosing.source.end - enclosing.source.start < found.source.end - found.source.start;
        if (isInside && isNarrower) {
            found = enclosing;
        }
    }

    return found?.moniker;
}
