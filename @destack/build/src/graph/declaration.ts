import { graph, type Package, type DeclarationDescription } from "@destack/package";
import { compareText } from "../build/serialization.ts";

/** A declaration or member of the build a relationship may name, with the declaration it belongs to. */
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
    /** Every declaration and member the build holds, by package, kind and name. */
    readonly #referents = new Map<string, Referent[]>();

    /** Locate every declaration and derive the package's declarations and edges. */
    constructor(source: Package, declarations: readonly DeclarationDescription[]) {
        // describe each declaration once across outputs with the members its symbols derive
        const located = new Map<graph.Moniker, Located>();
        for (const declaration of declarations) {
            const described = describe(declaration);
            const members = new Map(
                (declaration.symbols ?? []).flatMap(({ member }) =>
                    member === undefined
                        ? []
                        : [[`${member.name} ${member.kind}`, derive(described, member)] as const],
                ),
            );
            located.set(described.moniker, { declaration, described, members });
        }

        // index each declaration and member by its package, kind and name
        for (const { declaration, described, members } of located.values()) {
            const packageId = declaration.symbol.package.id;
            this.#index(packageId, { declaration: described });
            for (const member of members.values()) {
                this.#index(packageId, { declaration: member, parent: described });
            }
        }

        // keep the package's declarations and members with the edges their relationships draw
        for (const entry of located.values()) {
            if (entry.declaration.symbol.package.id === source.id) {
                this.#derive(entry);
            }
        }
    }

    /** List a module's declarations by moniker. */
    describe(path: string): graph.Declaration[] {
        return (this.#declarations.get(path) ?? []).toSorted((left, right) =>
            compareText(left.moniker, right.moniker),
        );
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

    /** Keep one of the package's declarations with its members, resolving the edges their relationships draw. */
    #derive(entry: Located): void {
        // resolve each relationship of the declaration and the members it derives
        const { declaration, described, members } = entry;
        const edges: graph.Edge[] = [];
        for (const symbol of declaration.symbols ?? []) {
            const from = symbol.member === undefined ? described : derive(described, symbol.member);
            for (const relationship of symbol.relationships) {
                for (const to of this.#resolve(
                    relationship.symbol,
                    declaration.symbol.package.id,
                )) {
                    edges.push({ from: from.moniker, to, kind: relationship.kind });
                }
            }
        }

        // keep the declarations and edges with the declaring module
        const path = declaration.symbol.symbol.module;
        const declarations = [described, ...members.values()];
        this.#declarations.set(path, [...(this.#declarations.get(path) ?? []), ...declarations]);
        this.#edges.set(path, [...(this.#edges.get(path) ?? []), ...edges]);
    }

    /** Name every declaration or member a reference matches, none when the build holds none. */
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
    // name the symbol, then the declaration as its kind at the symbol
    const location = {
        packageId: declaration.symbol.package.id,
        module: declaration.symbol.symbol.module,
        name: declaration.symbol.symbol.name,
    };

    return {
        moniker: graph.Moniker.of({ ...location, kind: declaration.kind }),
        symbol: graph.Moniker.of(location),
        kind: declaration.kind,
        package: declaration.package.id,
        name: declaration.name,
        description: declaration.description,
        ...(declaration.vocabulary === undefined ? {} : { vocabulary: declaration.vocabulary }),
    };
}

/** Describe a member a declaration's symbols derive, as a member of its symbol. */
function derive(
    parent: graph.Declaration,
    member: NonNullable<graph.MemberSymbol["member"]>,
): graph.Declaration {
    return {
        moniker: graph.Moniker.parse(`${parent.symbol}.${member.name}:${member.kind}`),
        symbol: parent.symbol,
        kind: member.kind,
        package: parent.package,
        name: member.name,
        description: member.description,
    };
}
