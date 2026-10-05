import { graph, type Package, type DeclarationDescription } from "@destack/package";
import type {
    ModuleDescription,
    SignatureDescription,
    SourceDeclaration,
    SymbolDescription,
    SymbolReference,
} from "@destack/package/code";
import type { SourceRange } from "@destack/package/source";
import { BuildError } from "../error/index.ts";
import { DeclarationGraph } from "./declaration.ts";
import { TestGraph, type TestSource } from "./test.ts";
import { compareText } from "../build/serialization.ts";

/** The symbol kind of each language declaration kind. */
const SYMBOL_KINDS: Readonly<Record<string, graph.Symbol["kind"]>> = {
    FunctionDeclaration: "function",
    FunctionExpression: "function",
    ArrowFunction: "function",
    ClassDeclaration: "class",
    ClassExpression: "class",
    InterfaceDeclaration: "interface",
    TypeAliasDeclaration: "type",
    TypeLiteral: "type",
    MappedType: "type",
    TypeParameter: "type",
    VariableDeclaration: "variable",
    BindingElement: "variable",
    Parameter: "variable",
    ExportAssignment: "variable",
    ObjectLiteralExpression: "variable",
    EnumDeclaration: "enum",
    ModuleDeclaration: "namespace",
    SourceFile: "namespace",
    ExportDeclaration: "namespace",
    NamespaceExport: "namespace",
    MethodDeclaration: "method",
    MethodSignature: "method",
    Constructor: "method",
    CallSignature: "method",
    ConstructSignature: "method",
    IndexSignature: "method",
    PropertyDeclaration: "property",
    PropertySignature: "property",
    PropertyAssignment: "property",
    ShorthandPropertyAssignment: "property",
    GetAccessor: "accessor",
    SetAccessor: "accessor",
    EnumMember: "member",
};

/** The longest printed signature line, TypeScript's hover truncation length (defaultMaximumTruncationLength). */
const SIGNATURE_LENGTH = 160;

/** The names the compiler gives anonymous symbols, such as function expressions and type literals. */
const INTERNAL_NAMES = new Set([
    "__function",
    "__object",
    "__type",
    "__class",
    "__call",
    "__new",
    "__index",
    "__constructor",
    "__export",
    "__computed",
    "__jsxAttributes",
]);

/** The symbol of a source range, by its first declaration. */
interface Enclosing {
    /** The symbol's moniker. */
    readonly moniker: graph.Moniker;
    /** The symbol's source range. */
    readonly source: SourceRange;
}

/** The edges of one module, each kept once. */
class Edges {
    /** The edges by their serialization. */
    readonly #edges = new Map<string, graph.Edge>();

    /** Add an edge, skipping an edge to its source or to an anonymous symbol. */
    add(from: graph.Moniker, kind: graph.Edge["kind"], to: graph.Moniker | undefined): void {
        if (to !== undefined && from !== to) {
            this.#edges.set(`${from} ${kind} ${to}`, { from, to, kind });
        }
    }

    /** List the edges by source, kind and target. */
    list(): graph.Edge[] {
        return [...this.#edges]
            .toSorted(([left], [right]) => compareText(left, right))
            .map(([, edge]) => edge);
    }
}

/** Describe each module's graph file from the checker's descriptions, the evaluated declarations and the tests. */
export function describeGraph(
    source: Package,
    modules: readonly ModuleDescription[],
    declarations: readonly DeclarationDescription[],
    tests: TestSource,
): graph.Module[] {
    // collect the symbols any module of the package exports
    const declared = new DeclarationGraph(source, declarations);
    const tested = new TestGraph(source, tests);
    const exported = new Set(
        modules.flatMap((module) => module.exports.map((entry) => monikerOf(source, entry.symbol))),
    );

    return modules.map((module) => describeModule(source, module, { declared, tested, exported }));
}

/** The package-wide graph inputs each module's graph file draws on. */
interface PackageGraph {
    /** The package's declarations and their edges. */
    readonly declared: DeclarationGraph;
    /** The package's tests. */
    readonly tested: TestGraph;
    /** The symbols any module of the package exports. */
    readonly exported: ReadonlySet<graph.Moniker>;
}

/** Describe one module's symbols, declarations and outgoing edges. */
function describeModule(
    source: Package,
    module: ModuleDescription,
    { declared, tested, exported }: PackageGraph,
): graph.Module {
    // name the module and its exports
    const moniker = graph.Moniker.of({ packageId: source.id, module: module.path });
    const exports = module.exports.map((entry) => ({
        name: entry.name,
        symbol: monikerOf(source, entry.symbol),
        isTypeOnly: entry.isTypeOnly,
    }));

    // describe each named symbol and member once, with the edges of their types
    const symbols = new ModuleSymbols(source, module, exported);
    for (const symbol of module.symbols) {
        symbols.add(symbol);
    }
    const described = symbols.list();
    const edges = symbols.edges;

    // attribute calls and globals to their innermost enclosing symbol or test
    const tests = tested.describe(module.path);
    const covering = new Set(tests.map((test) => test.declaration.moniker));
    const enclosings = [
        ...described,
        ...tests.map((test) => ({ moniker: test.declaration.moniker, source: test.source })),
    ];
    const attribute = (range: SourceRange, kind: "calls" | "references", to?: graph.Moniker) => {
        const from = enclose(enclosings, range) ?? moniker;
        edges.add(from, kind, to);
        if (to !== undefined && covering.has(from) && isCovered(source, tested, to)) {
            edges.add(from, "covers", to);
            for (const declaration of declared.at(to)) {
                edges.add(from, "covers", declaration);
            }
        }
    };
    for (const description of module.errors) {
        for (const call of description.calls) {
            if (call.target !== undefined) {
                attribute(call.source, "calls", symbols.name(call.target));
            }
        }
    }
    for (const global of module.globals) {
        if (global.symbol !== undefined) {
            attribute(global.source, "references", symbols.name(global.symbol));
        }
    }

    // depend on each resolved import
    for (const imported of module.imports) {
        if (imported.target !== undefined) {
            edges.add(moniker, "depends", symbols.name(imported.target));
        }
    }

    // describe the module's declarations and tests with their edges
    const declarations = [
        ...declared.describe(module.path),
        ...tests.map((test) => test.declaration),
    ].toSorted((left, right) => compareText(left.moniker, right.moniker));
    for (const edge of declared.edges(module.path)) {
        edges.add(edge.from, edge.kind, edge.to);
    }

    return graph.Module.parse({
        path: module.path,
        digest: module.source.digest,
        imports: module.imports.map((imported) => imported.specifier),
        exports,
        symbols: described,
        declarations,
        edges: edges.list(),
    });
}

/** The named symbols and members of one module, with the edges of their types. */
class ModuleSymbols {
    /** The package declaring the module. */
    readonly source: Package;
    /** The described module. */
    readonly module: ModuleDescription;
    /** The symbols any module of the package exports. */
    readonly exported: ReadonlySet<graph.Moniker>;
    /** The module's edges. */
    readonly edges = new Edges();
    /** The first description of each symbol, by moniker. */
    readonly #symbols = new Map<graph.Moniker, graph.Symbol>();

    /** Collect the symbols of a module. */
    constructor(source: Package, module: ModuleDescription, exported: ReadonlySet<graph.Moniker>) {
        this.source = source;
        this.module = module;
        this.exported = exported;
    }

    /** Name a referenced symbol, absent for an anonymous one. */
    name(reference: SymbolReference): graph.Moniker | undefined {
        const name = "symbol" in reference ? reference.symbol.name : reference.name;

        return isAnonymous(name) ? undefined : monikerOf(this.source, reference);
    }

    /** Describe a symbol and its members, leaving anonymous types and functions to their enclosing symbol. */
    add(symbol: SymbolDescription): void {
        // name the symbol
        if (isAnonymous(symbol.name)) {
            return;
        }
        const { source, module } = this;
        const self = graph.Moniker.of({
            packageId: source.id,
            module: module.path,
            name: symbol.name,
        });
        const isExported = this.exported.has(self);

        // describe the symbol and the edges of its types and heritage
        this.#keep({
            moniker: self,
            kind: kindOf(symbol.declarations, `${module.path}#${symbol.name}`),
            source: first(symbol.declarations).source,
            signature: truncate(printSymbol(symbol)),
            ...commentOf(symbol.documentation.text),
            isExported,
        });
        for (const reference of symbolReferences(symbol)) {
            this.edges.add(self, "references", this.name(reference));
        }
        for (const declaration of symbol.declarations) {
            for (const heritage of declaration.heritage) {
                for (const reference of heritage.type.references) {
                    this.edges.add(self, "implements", this.name(reference));
                }
            }
        }

        // describe each named member
        for (const member of symbol.members) {
            if (!isAnonymous(member.name)) {
                this.#member(symbol, member, self, isExported);
            }
        }
    }

    /** List the described symbols by moniker. */
    list(): graph.Symbol[] {
        return [...this.#symbols.values()].toSorted((left, right) =>
            compareText(left.moniker, right.moniker),
        );
    }

    /** Describe a member of a symbol with the edges of its type and signatures. */
    #member(
        symbol: SymbolDescription,
        member: SymbolDescription["members"][number],
        self: graph.Moniker,
        isExported: boolean,
    ): void {
        // describe the member
        const nested = graph.Moniker.of({
            packageId: this.source.id,
            module: this.module.path,
            name: symbol.name,
            member: member.name,
        });
        this.#keep({
            moniker: nested,
            kind: kindOf(member.declarations, `${self}.${member.name}`),
            source: first(member.declarations).source,
            signature: truncate(printMember(member)),
            ...commentOf(member.documentation.text),
            isExported,
        });

        // reference the declarations its type and signatures name
        const references = [
            ...member.type.references,
            ...member.signatures.flatMap(signatureReferences),
        ];
        for (const reference of references) {
            this.edges.add(nested, "references", this.name(reference));
        }
    }

    /** Keep the first description of a symbol the checker describes twice, as a member and as a symbol. */
    #keep(symbol: graph.Symbol): void {
        if (!this.#symbols.has(symbol.moniker)) {
            this.#symbols.set(symbol.moniker, symbol);
        }
    }
}

/** Name a referenced symbol by its moniker, or its module's moniker for a module. */
export function monikerOf(source: Package, reference: SymbolReference): graph.Moniker {
    // locate the package and symbol
    let packageId: string;
    let symbol: { readonly module: string; readonly name: string };
    if ("compiler" in reference) {
        packageId = `npm:${reference.compiler.name}`;
        symbol = reference.symbol;
    } else if ("package" in reference) {
        packageId =
            "id" in reference.package ? reference.package.id : `npm:${reference.package.name}`;
        symbol = reference.symbol;
    } else {
        packageId = source.id;
        symbol = reference;
    }

    // name a module by its path alone
    return graph.Moniker.of({
        packageId,
        module: symbol.module,
        ...(symbol.name === "*" ? {} : { name: symbol.name }),
    });
}

/** Report whether a symbol is one of the package's own outside its test modules. */
function isCovered(source: Package, tested: TestGraph, symbol: graph.Moniker): boolean {
    // keep the package's own symbols
    const prefix = `${source.id}/`;
    if (!symbol.startsWith(prefix)) {
        return false;
    }

    // leave symbols of the test modules themselves
    const path = symbol.slice(prefix.length).split("#")[0] ?? "";

    return !tested.has(path);
}

/** Report whether a qualified name passes through a symbol the compiler names internally, such as `__function`. */
function isAnonymous(name: string): boolean {
    return name.split(".").some((part) => INTERNAL_NAMES.has(part));
}

/** Read the symbol kind of a symbol's first declaration. */
function kindOf(declarations: readonly SourceDeclaration[], name: string): graph.Symbol["kind"] {
    const kind = SYMBOL_KINDS[first(declarations).kind];
    if (kind === undefined) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `unknown declaration kind ${first(declarations).kind}: ${name}`,
        );
    }

    return kind;
}

/** Read a symbol's first declaration, which the checker requires. */
function first(declarations: readonly SourceDeclaration[]): SourceDeclaration {
    const [declaration] = declarations;
    if (declaration === undefined) {
        throw new BuildError("INSPECTION_FAILED", "a symbol has no declaration");
    }

    return declaration;
}

/** Print a symbol's declaration, one line per overload. */
function printSymbol(symbol: SymbolDescription): string {
    // read the first declaration's generics, heritage and kind
    const declaration = first(symbol.declarations);
    const generics = printGenerics(declaration);
    const heritage = declaration.heritage
        .map((clause) => ` ${clause.relation} ${clause.type.text}`)
        .join("");
    const kind = SYMBOL_KINDS[declaration.kind];

    // print callables by overload, types by their definition and values by their type
    if (kind === "function") {
        return symbol.signatures
            .map((signature) => `function ${symbol.name}${printSignature(signature)}`)
            .join("\n");
    } else if (kind === "class" || kind === "interface") {
        return `${kind} ${symbol.name}${generics}${heritage}`;
    } else if (kind === "type") {
        return `type ${symbol.name}${generics} = ${symbol.declaredType?.text ?? ""}`;
    } else if (kind === "enum" || kind === "namespace") {
        return `${kind} ${symbol.name}`;
    } else {
        return `${symbol.name}: ${symbol.valueType?.text ?? symbol.declaredType?.text ?? ""}`;
    }
}

/** Truncate each line of a printed signature, leaving complete types to detail queries. */
function truncate(signature: string): string {
    return signature
        .split("\n")
        .map((line) =>
            line.length > SIGNATURE_LENGTH ? `${line.slice(0, SIGNATURE_LENGTH - 1)}…` : line,
        )
        .join("\n");
}

/** Print a member's declaration, one line per overload of a method. */
function printMember(member: SymbolDescription["members"][number]): string {
    const kind = SYMBOL_KINDS[first(member.declarations).kind];
    const optional = member.isOptional ? "?" : "";

    // print methods by overload, enum members by name and the rest by type
    if (kind === "method") {
        return member.signatures
            .map((signature) => `${member.name}${optional}${printSignature(signature)}`)
            .join("\n");
    } else if (kind === "member") {
        return member.name;
    } else {
        return `${member.name}${optional}: ${member.type.text}`;
    }
}

/** Print a signature without its trailing semicolon. */
function printSignature(signature: SignatureDescription): string {
    return signature.text.endsWith(";") ? signature.text.slice(0, -1) : signature.text;
}

/** Print a declaration's generic parameters. */
function printGenerics(declaration: SourceDeclaration): string {
    if (!declaration.typeParameters.length) {
        return "";
    }
    const parameters = declaration.typeParameters.map((parameter) => {
        const constraint = parameter.constraint ? ` extends ${parameter.constraint.text}` : "";
        const fallback = parameter.default ? ` = ${parameter.default.text}` : "";

        return `${parameter.name}${constraint}${fallback}`;
    });

    return `<${parameters.join(", ")}>`;
}

/** Read the first sentence of documentation, absent without documentation. */
function commentOf(text: string): { comment?: string } {
    const sentence = /^[\s\S]*?[.!?](?=\s|$)/u.exec(text.trim())?.[0] ?? text.trim();

    return sentence ? { comment: sentence.replaceAll(/\s+/gu, " ") } : {};
}

/** List the declarations a symbol's declarations, signatures and indexes name. */
function symbolReferences(symbol: SymbolDescription): SymbolReference[] {
    return [
        ...(symbol.declaredType?.references ?? []),
        ...(symbol.valueType?.references ?? []),
        ...symbol.typeSignatures.flatMap(signatureReferences),
        ...symbol.signatures.flatMap(signatureReferences),
        ...symbol.indexes.flatMap((index) => [...index.key.references, ...index.value.references]),
        ...symbol.declarations.flatMap((declaration) =>
            declaration.typeParameters.flatMap(parameterReferences),
        ),
    ];
}

/** List the declarations a signature names. */
function signatureReferences(signature: SignatureDescription): SymbolReference[] {
    return [
        ...signature.returns.references,
        ...(signature.receiver?.references ?? []),
        ...signature.parameters.flatMap((parameter) => parameter.references),
        ...signature.typeParameters.flatMap(parameterReferences),
    ];
}

/** List the declarations a generic parameter names. */
function parameterReferences(
    parameter: SourceDeclaration["typeParameters"][number],
): SymbolReference[] {
    return [...(parameter.constraint?.references ?? []), ...(parameter.default?.references ?? [])];
}

/** Find the innermost symbol whose source range contains a range. */
function enclose(symbols: readonly Enclosing[], range: SourceRange): graph.Moniker | undefined {
    let found: Enclosing | undefined;
    for (const symbol of symbols) {
        const isInside =
            symbol.source.file === range.file &&
            symbol.source.start <= range.start &&
            range.end <= symbol.source.end;
        const isInner =
            found === undefined ||
            symbol.source.end - symbol.source.start < found.source.end - found.source.start;
        if (isInside && isInner) {
            found = symbol;
        }
    }

    return found?.moniker;
}
