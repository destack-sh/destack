import { Text } from "@destack/schema";
import { graph, type PackageVersion, type DeclarationDescription } from "@destack/package";

/** A declaration or member a relationship may refer to, with its declaration. */
interface Referent {
    /** The declaration or member. */
    readonly declaration: graph.Declaration;
    /** The declaration a member belongs to, absent for a declaration. */
    readonly parent?: graph.Declaration;
}

/** A declaration of the build with the members its symbols derive. */
interface Located {
    /** The evaluated declaration. */
    readonly declaration: DeclarationDescription;
    /** The declaration's graph form. */
    readonly described: graph.Declaration;
    /** The members its symbols derive, by name and kind. */
    readonly members: ReadonlyMap<string, graph.Declaration>;
}

/** The declarations of a build and the edges between them, from the symbols their kinds list. */
export class DeclarationGraph {
    /** The package's declarations and the members they derive, by module path. */
    readonly #declarations = new Map<string, graph.Declaration[]>();
    /** The edges from the package's declarations, by module path. */
    readonly #edges = new Map<string, graph.Edge[]>();
    /** The package's declarations, by the symbol declaring them. */
    readonly #symbols = new Map<graph.Moniker, graph.Moniker[]>();
    /** Every declaration and member the build has, by package, kind and name. */
    readonly #referents = new Map<string, Referent[]>();

    /** Hold no declarations, which describe and read fill. */
    private constructor() {}

    /** Locate every declaration and derive the package's declarations and edges. */
    static describe(
        source: PackageVersion,
        declarations: readonly DeclarationDescription[],
    ): DeclarationGraph {
        // describe each declaration once across outputs with the members its symbols derive
        const graphed = new DeclarationGraph();
        const located = new Map<graph.Moniker, Located>();
        for (const declaration of declarations) {
            const declared = describe(declaration);
            const members = new Map(
                (declaration.symbols ?? []).flatMap(({ member }) =>
                    member === undefined
                        ? []
                        : [
                              [
                                  `${member.name} ${member.kind}`,
                                  graph.Declaration.member(declared, member),
                              ] as const,
                          ],
                ),
            );
            located.set(declared.moniker, { declaration, described: declared, members });
        }

        // index each declaration and member by its package, kind and name
        for (const { declaration, described, members } of located.values()) {
            const packageId = declaration.symbol.package.id;
            graphed.#index(packageId, { declaration: described });
            for (const member of members.values()) {
                graphed.#index(packageId, { declaration: member, parent: described });
            }
        }

        // store the package's declarations and members with the edges their relationships draw
        for (const entry of located.values()) {
            if (entry.declaration.symbol.package.id === source.id) {
                graphed.#derive(entry);
            }
        }

        return graphed;
    }

    /** Read a build's stored declarations, members and relationships as its graph. */
    static read(
        declarations: readonly graph.Declaration[],
        relationships: readonly graph.Edge[],
    ): DeclarationGraph {
        // index each stored declaration and member by its module and symbol
        const read = new DeclarationGraph();
        for (const declaration of declarations) {
            const path = graph.Moniker.module(declaration.symbol);
            read.#declarations.set(path, [...(read.#declarations.get(path) ?? []), declaration]);
            read.#symbols.set(declaration.symbol, [
                ...(read.#symbols.get(declaration.symbol) ?? []),
                declaration.moniker,
            ]);
        }

        // index each relationship by the module of the declaration it leaves
        for (const edge of relationships) {
            const path = graph.Moniker.module(edge.from);
            read.#edges.set(path, [...(read.#edges.get(path) ?? []), edge]);
        }

        return read;
    }

    /** List the relationships between the package's declarations and members, as edges. */
    relationships(): graph.Edge[] {
        return [...this.#edges.values()]
            .flat()
            .toSorted((left, right) =>
                Text.compare(
                    `${left.from} ${left.kind} ${left.to}`,
                    `${right.from} ${right.kind} ${right.to}`,
                ),
            );
    }

    /** List the package's declarations and the members they derive, by moniker. */
    list(): graph.Declaration[] {
        return [...this.#declarations.values()]
            .flat()
            .toSorted((left, right) => Text.compare(left.moniker, right.moniker));
    }

    /** List a module's declarations by moniker. */
    describe(path: string): graph.Declaration[] {
        return (this.#declarations.get(path) ?? []).toSorted((left, right) =>
            Text.compare(left.moniker, right.moniker),
        );
    }

    /** List the package's declarations at a symbol. */
    at(symbol: graph.Moniker): readonly graph.Moniker[] {
        return this.#symbols.get(symbol) ?? [];
    }

    /** List the edges from a module's declarations. */
    edges(path: string): readonly graph.Edge[] {
        return this.#edges.get(path) ?? [];
    }

    /** Index a declaration or member under its package, kind and name. */
    #index(packageId: string, referent: Referent): void {
        const key = `${packageId} ${referent.declaration.kind} ${referent.declaration.name}`;
        this.#referents.set(key, [...(this.#referents.get(key) ?? []), referent]);
    }

    /** Store a package declaration with its members, resolving their relationship edges. */
    #derive(entry: Located): void {
        // resolve each relationship of the declaration and the members it derives
        const { declaration, described, members } = entry;
        const edges: graph.Edge[] = [];
        for (const symbol of declaration.symbols ?? []) {
            const from =
                symbol.member === undefined
                    ? described
                    : graph.Declaration.member(described, symbol.member);
            for (const relationship of symbol.relationships) {
                for (const to of this.#resolve(
                    relationship.symbol,
                    declaration.symbol.package.id,
                )) {
                    edges.push({ from: from.moniker, to, kind: relationship.kind });
                }
            }
        }

        // store the declarations and edges with the declaring module
        const path = declaration.symbol.symbol.module;
        const declarations = [described, ...members.values()];
        this.#declarations.set(path, [...(this.#declarations.get(path) ?? []), ...declarations]);
        this.#edges.set(path, [...(this.#edges.get(path) ?? []), ...edges]);
        for (const stored of declarations) {
            this.#symbols.set(stored.symbol, [
                ...(this.#symbols.get(stored.symbol) ?? []),
                stored.moniker,
            ]);
        }
    }

    /** List every declaration or member a reference matches, none when the build has none. */
    #resolve(reference: graph.Target, packageId: string): graph.Moniker[] {
        // match the kind and name in the reference's package, then the declaration a member belongs to
        const key = `${reference.packageId ?? packageId} ${reference.kind} ${reference.name}`;
        const parent = reference.parent;

        return (this.#referents.get(key) ?? [])
            .filter(
                (referent) =>
                    parent === undefined ||
                    (referent.parent?.kind === parent.kind && referent.parent.name === parent.name),
            )
            .map((referent) => referent.declaration.moniker);
    }
}

/** Describe an evaluated declaration at its symbol. */
function describe(declaration: DeclarationDescription): graph.Declaration {
    // key the symbol, then the declaration as its kind at the symbol
    const location = {
        packageId: declaration.symbol.package.id,
        module: declaration.symbol.symbol.module,
        name: declaration.symbol.symbol.name,
    };

    return graph.Declaration.at(graph.Moniker.of(location), {
        kind: declaration.kind,
        kindPackageId: declaration.package.id,
        name: declaration.name,
        description: declaration.description,
        ...(declaration.provisioned === undefined ? {} : { provisioned: declaration.provisioned }),
        ...(declaration.vocabulary === undefined ? {} : { vocabulary: declaration.vocabulary }),
    });
}
