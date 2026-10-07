import { aligned, present, Text } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import {
    NodeBuilderFlags,
    type NodeHandle,
    type Project,
    type Signature,
    SignatureKind,
    type Symbol as TypeScriptSymbol,
    SymbolFlags,
    type Type,
} from "typescript/unstable/async";
import {
    isHeritageClause,
    isIdentifier,
    isModifierLike,
    isParameterDeclaration,
    isTypeAliasDeclaration,
    isTypeParameterDeclaration,
    type Node,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import type {
    SourceDeclaration,
    SignatureDescription,
    SymbolDescription,
    SymbolReference,
    TypeDescription,
} from "@destack/package/code";
import type { SourceRange } from "@destack/package/source";
import { isAuthored, relativePath } from "../source/dependency.ts";

/** Named declarations retained as references instead of expanded structural types. */
const REFERENCE_FLAGS =
    SymbolFlags.Class |
    SymbolFlags.Interface |
    SymbolFlags.TypeAlias |
    SymbolFlags.Enum |
    SymbolFlags.Function |
    SymbolFlags.ValueModule;

/** The name the compiler gives a private class member, `__#<class symbol id>@#name`, whose id numbers symbols per program. */
const PRIVATE_NAME = /^__#\d+@(#.+)$/u;

/** One step of a reference walk: a type's named declaration and the types it leads to. */
interface TypeStep {
    /** The named declaration, absent for a structural type. */
    readonly reference: SymbolReference | undefined;
    /** The generic arguments, and the constituents of a structural type. */
    readonly next: readonly Type[];
}

/** Extract public API descriptions within one compiler snapshot. */
export class SymbolInspector {
    /** The compiler project being inspected. */
    readonly project: Project;
    /** Resolve named declarations and enqueue local dependencies. */
    readonly reference: (symbol: TypeScriptSymbol) => Promise<SymbolReference>;
    /** The package directory. */
    readonly root: string;
    /** The source file of each declaration path, fetched once. */
    readonly #sources = new Map<string, Promise<SourceFile | undefined>>();
    /** The step of each type a walk has visited, shared by every walk of the inspection. */
    readonly #steps = new Map<number, Promise<TypeStep>>();
    /** The package-relative path of each compiler file. */
    readonly #paths = new Map<string, string>();

    /** Associate compiler results with package declarations and source files. */
    constructor(
        project: Project,
        reference: (symbol: TypeScriptSymbol) => Promise<SymbolReference>,
        root: string,
    ) {
        this.project = project;
        this.reference = reference;
        this.root = root;
    }

    /** Resolve an alias to the symbol it names, keeping any other symbol. */
    async original(symbol: TypeScriptSymbol): Promise<TypeScriptSymbol> {
        return symbol.flags & SymbolFlags.Alias
            ? await this.project.checker.getAliasedSymbol(symbol)
            : symbol;
    }

    /** Describe a symbol's type and value namespaces independently. */
    async describe(symbol: TypeScriptSymbol, name: string): Promise<SymbolDescription> {
        // collect the package's merged declarations and their documentation
        const nodes = await this.#authored(symbol);
        const result: SymbolDescription = {
            name,
            declarations: await Promise.all(nodes.map((node) => this.declaration(node))),
            documentation: await this.documentation(symbol),
            signatures: [],
            typeSignatures: [],
            members: [],
            exports: [],
            indexes: [],
        };

        // describe the type namespace
        const declared =
            symbol.flags & SymbolFlags.Type
                ? await this.project.checker.getDeclaredTypeOfSymbol(symbol)
                : undefined;
        if (declared !== undefined) {
            await this.#describeType(result, declared, nodes);
        }

        // describe the value namespace
        if (symbol.flags & SymbolFlags.Value) {
            const type = await this.project.checker.getTypeOfSymbol(symbol);
            if (!type) {
                throw new BuildError("INSPECTION_FAILED", `missing value type: ${symbol.name}`);
            }
            result.valueType = await this.type(type, nodes[0]);
            result.signatures = await this.signatures(type, nodes[0]);
        }

        // describe a module's exports and every member the package declares once
        const members = await this.#members(symbol, declared, nodes, result);
        const declarations = await Promise.all(members.map((member) => this.#authored(member)));
        const described = members.flatMap((member, index) => {
            const authored = aligned(declarations, index);

            return authored.length ? [this.member(member, symbol, authored)] : [];
        });
        result.members = await Promise.all(described);

        return result;
    }

    /** Describe a symbol's declared type, call signatures, index signatures and authored alias expression. */
    async #describeType(
        result: SymbolDescription,
        declared: Type,
        nodes: readonly Node[],
    ): Promise<void> {
        // describe the declared type and its signatures
        const location = nodes[0];
        result.declaredType = await this.type(declared, location);
        result.typeSignatures = await this.signatures(declared, location);

        // retain index signatures
        for (const index of await this.project.checker.getIndexInfosOfType(declared)) {
            result.indexes.push({
                key: await this.type(index.keyType, location),
                value: await this.type(index.valueType, location),
                isReadonly: index.isReadonly,
            });
        }

        // preserve the authored alias expression
        const alias = nodes.find(isTypeAliasDeclaration);
        if (alias) {
            result.declaredType.text = alias.type.getText();
            result.declaredType.references = await this.references(alias.type);
        }
    }

    /** List a symbol's declared members once, recording a module's exports instead of its members. */
    async #members(
        symbol: TypeScriptSymbol,
        declared: Type | undefined,
        nodes: readonly Node[],
        result: SymbolDescription,
    ): Promise<TypeScriptSymbol[]> {
        // retain declared members, including accessors and class static members
        const members = [...(await symbol.getMembers()).values()];
        const exports = [...(await symbol.getExports()).values()];
        if (symbol.flags & SymbolFlags.Module) {
            for (const exported of exports) {
                const original = await this.original(exported);
                result.exports.push({
                    name: exported.name,
                    symbol: await this.reference(original),
                });
            }
        } else {
            members.push(...exports);
        }

        // collect members declared in object type aliases
        const isObjectAlias = nodes.some(
            (node) => isTypeAliasDeclaration(node) && node.type.kind === SyntaxKind.TypeLiteral,
        );
        if (declared && isObjectAlias) {
            const owner = await declared.getSymbol();
            if (owner && !(owner.flags & (SymbolFlags.Class | SymbolFlags.Interface))) {
                members.push(...(await owner.getMembers()).values());
            }
        }

        // keep each declared member once, leaving out generic parameters, signatures and constructors
        const seen = new Set<number>();
        const omitted = SymbolFlags.TypeParameter | SymbolFlags.Signature | SymbolFlags.Constructor;

        return members.filter((member) => {
            const isNew = !seen.has(member.id) && member.declarations.length > 0;
            seen.add(member.id);

            return isNew && !(member.flags & omitted);
        });
    }

    /** Describe a member from its declarations in the package, with its type, overloads and documentation. */
    async member(
        symbol: TypeScriptSymbol,
        parent: TypeScriptSymbol,
        declarations: readonly Node[],
    ): Promise<SymbolDescription["members"][number]> {
        // inspect the original declaration in its type or value namespace
        const location = declarations[0];
        const type = await this.#memberType(await this.original(symbol), location);
        if (type === undefined || type.isErrorType()) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `cannot inspect member: ${parent.name}.${symbol.name} (${symbol.flags})`,
            );
        }

        // describe the declarations, type, overloads and documentation together
        const [described, typed, signatures, documentation] = await Promise.all([
            Promise.all(declarations.map((node) => this.declaration(node))),
            this.type(type, location),
            this.signatures(type, location),
            this.documentation(symbol),
        ]);

        return {
            name: declaredName(symbol),
            declarations: described,
            isOptional: Boolean(symbol.flags & SymbolFlags.Optional),
            type: typed,
            signatures,
            documentation,
        };
    }

    /** Read a member's declared type, or its value type at its declaration. */
    async #memberType(
        original: TypeScriptSymbol,
        location: Node | undefined,
    ): Promise<Type | undefined> {
        // read a type member's declared type
        if (original.flags & SymbolFlags.Type) {
            return await this.project.checker.getDeclaredTypeOfSymbol(original);
        }
        // read a value member's type at its declaration
        else if (location !== undefined) {
            return await this.project.checker.getTypeOfSymbolAtLocation(original, location);
        }
        // read nothing without a declaration
        else {
            return undefined;
        }
    }

    /** Describe one source declaration independently of merged symbols. */
    async declaration(node: Node): Promise<SourceDeclaration> {
        // retain source ranges independently for merged declarations
        const result: SourceDeclaration = {
            kind: SyntaxKind[node.kind],
            source: this.range(node),
            modifiers: [],
            typeParameters: [],
            heritage: [],
        };

        // retain modifiers and generic parameters on their original declarations
        if ("modifiers" in node) {
            result.modifiers = nodeList(node.modifiers, isModifierLike).map((modifier) =>
                modifier.getText(),
            );
        }

        // preserve generic constraints and defaults
        if ("typeParameters" in node) {
            for (const parameter of nodeList(node.typeParameters, isTypeParameterDeclaration)) {
                result.typeParameters.push(await this.parameterType(parameter));
            }
        }

        // retain each declared base type and its named references
        if ("heritageClauses" in node) {
            for (const clause of nodeList(node.heritageClauses, isHeritageClause)) {
                for (const expression of clause.types) {
                    result.heritage.push({
                        relation:
                            clause.token === SyntaxKind.ExtendsKeyword ? "extends" : "implements",
                        type: {
                            text: expression.getText(),
                            references: await this.references(expression),
                        },
                    });
                }
            }
        }

        return result;
    }

    /** Resolve names in an authored type expression, preserving aliases and type operators. */
    async references(node: Node): Promise<SymbolReference[]> {
        // collect identifier nodes before querying the compiler
        const identifiers: Node[] = [];
        const pending = [node];
        for (let current = pending.pop(); current !== undefined; current = pending.pop()) {
            if (isIdentifier(current)) {
                identifiers.push(current);
            }
            current.forEachChild((child) => {
                pending.push(child);
            });
        }

        // resolve identifiers in one compiler request
        const result = new Map<string, SymbolReference>();
        for (const symbol of await this.project.checker.getSymbolAtLocation(identifiers)) {
            if (!symbol) {
                continue;
            }

            // follow aliases to the referenced declaration
            const original = await this.original(symbol);
            if (!(original.flags & REFERENCE_FLAGS)) {
                continue;
            }

            const reference = await this.reference(original);
            result.set(JSON.stringify(reference), reference);
        }

        return [...result.values()];
    }

    /** Describe an expression and the named declarations needed to understand it. */
    async type(type: Type, location: Node | undefined): Promise<TypeDescription> {
        if (type.isErrorType()) {
            throw new BuildError("INSPECTION_FAILED", "cannot inspect an unresolved type");
        }

        // render the expression as hovers do, truncated, while walking its references
        const [text, references] = await Promise.all([
            this.project.checker.typeToString(type, location),
            this.#references(type),
        ]);

        return { text, references };
    }

    /** Walk a type breadth first through the steps shared by every walk, stopping at named declarations. */
    async #references(type: Type): Promise<SymbolReference[]> {
        // fetch each level's steps together and visit each type once
        const references = new Map<string, SymbolReference>();
        const visited = new Set([type.id]);
        let level = [type];
        while (level.length) {
            const steps = await Promise.all(level.map((current) => this.#step(current)));
            level = [];
            for (const step of steps) {
                // retain the named declaration
                if (step.reference !== undefined) {
                    references.set(JSON.stringify(step.reference), step.reference);
                }

                // queue the types the step leads to
                for (const next of step.next) {
                    if (!visited.has(next.id)) {
                        visited.add(next.id);
                        level.push(next);
                    }
                }
            }
        }

        return [...references.entries()]
            .toSorted(([left], [right]) => Text.compare(left, right))
            .map(([, reference]) => reference);
    }

    /** Read a type's step once per inspection. */
    #step(type: Type): Promise<TypeStep> {
        let step = this.#steps.get(type.id);
        if (step === undefined) {
            step = this.#walk(type);
            this.#steps.set(type.id, step);
        }

        return step;
    }

    /** Read a type's named declaration and the types it leads to, expanding only unnamed types. */
    async #walk(type: Type): Promise<TypeStep> {
        // name a declared class, interface, alias, enum, function or namespace
        const symbol = (await type.getAliasSymbol()) ?? (await type.getSymbol());
        const isNamed =
            symbol !== undefined &&
            symbol.declarations.length > 0 &&
            Boolean(symbol.flags & REFERENCE_FLAGS);

        // follow the generic arguments of a named type or a reference, such as a tuple's elements
        const isReference = type.isTypeReference();
        const [reference, aliasArguments, typeArguments, expanded] = await Promise.all([
            isNamed ? this.reference(symbol) : undefined,
            type.getAliasTypeArguments(),
            isReference ? this.project.checker.getTypeArguments(type) : [],
            isNamed || isReference ? [] : this.#expand(type),
        ]);

        return { reference, next: [...aliasArguments, ...typeArguments, ...expanded] };
    }

    /** Collect constituent types from an unnamed type expression. */
    async #expand(type: Type): Promise<readonly Type[]> {
        // expand composite expressions
        if (type.isUnionType() || type.isIntersectionType() || type.isTemplateLiteralType()) {
            return present(await type.getTypes(), "the constituents of a composite type");
        }
        // inspect the object and key of an indexed access
        else if (type.isIndexedAccessType()) {
            return await Promise.all([type.getObjectType(), type.getIndexType()]);
        }
        // follow type operators to their target
        else if (type.isIndexType() || type.isStringMappingType()) {
            return [await type.getTarget()];
        }
        // inspect both conditional branches
        else if (type.isConditionalType()) {
            return await Promise.all([
                type.getCheckType(),
                type.getExtendsType(),
                type.getTrueType(),
                type.getFalseType(),
            ]);
        }
        // retain substitution constraints
        else if (type.isSubstitutionType()) {
            return await Promise.all([type.getBaseType(), type.getConstraint()]);
        }
        // inspect structural members and callable signatures
        else if (type.isObjectType()) {
            const [properties, calls, constructs] = await Promise.all([
                this.project.checker.getPropertiesOfType(type),
                this.project.checker.getSignaturesOfType(type, SignatureKind.Call),
                this.project.checker.getSignaturesOfType(type, SignatureKind.Construct),
            ]);
            const [propertyTypes, signatureTypes] = await Promise.all([
                this.#typesOf(properties),
                Promise.all(
                    [...calls, ...constructs].map((signature) => this.#signatureTypes(signature)),
                ),
            ]);

            return [...propertyTypes, ...signatureTypes.flat()];
        }

        return [];
    }

    /** Read a signature's return type and parameter types. */
    async #signatureTypes(signature: Signature): Promise<Type[]> {
        const [returns, parameters] = await Promise.all([
            this.project.checker.getReturnTypeOfSignature(signature),
            signature.getParameters().then((symbols) => this.#typesOf(symbols)),
        ]);
        if (!returns) {
            throw new BuildError("INSPECTION_FAILED", "missing signature return type");
        }

        return [returns, ...parameters];
    }

    /** Read the types of symbols in one compiler request, requiring each. */
    async #typesOf(symbols: readonly TypeScriptSymbol[]): Promise<Type[]> {
        if (!symbols.length) {
            return [];
        }
        const types = await this.project.checker.getTypeOfSymbol(symbols);

        return types.map((type, index) => {
            if (type === undefined) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `missing type: ${aligned(symbols, index).name}`,
                );
            }

            return type;
        });
    }

    /** Describe each callable and constructable overload in compiler order. */
    async signatures(type: Type, location: Node | undefined): Promise<SignatureDescription[]> {
        const signatures = await Promise.all(
            [SignatureKind.Call, SignatureKind.Construct].map((kind) =>
                this.project.checker.getSignaturesOfType(type, kind),
            ),
        );

        return await Promise.all(
            signatures.flat().map((signature) => this.signature(signature, location)),
        );
    }

    /** Describe a callable signature, including receiver and rest parameters. */
    async signature(
        signature: Signature,
        location: Node | undefined,
    ): Promise<SignatureDescription> {
        // render the signature for display, truncated as type printing does
        const declaration =
            signature.declaration === undefined ? location : await this.node(signature.declaration);
        const node = await this.project.checker.signatureToSignatureDeclaration(
            signature,
            signature.isConstruct ? SyntaxKind.ConstructSignature : SyntaxKind.CallSignature,
            declaration,
            NodeBuilderFlags.UseAliasDefinedOutsideCurrentScope | NodeBuilderFlags.IgnoreErrors,
        );
        const returns = await this.project.checker.getReturnTypeOfSignature(signature);
        if (!node || !returns) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `cannot describe a callable signature: ${declaration?.getText()}`,
            );
        }

        // retain the rendered signature and structured return type
        const result: SignatureDescription = {
            kind: signature.isConstruct ? "construct" : "call",
            text: await this.project.emitter.printNode(node),
            typeParameters: [],
            parameters: [],
            returns: await this.type(returns, declaration),
        };

        // describe the generic parameters and the parameters
        result.typeParameters = await this.#typeParameters(signature);
        result.parameters = await this.#parameters(signature);

        // retain an explicit this parameter
        const receiver = await signature.getThisParameter();
        if (receiver) {
            const type = await this.project.checker.getTypeOfSymbol(receiver);
            if (!type) {
                throw new BuildError("INSPECTION_FAILED", "missing receiver type");
            }
            result.receiver = await this.type(type, declaration);
        }

        return result;
    }

    /** Describe a signature's generic constraints and defaults from their original declarations. */
    async #typeParameters(
        signature: Signature,
    ): Promise<SourceDeclaration["typeParameters"][number][]> {
        return await Promise.all(
            (await signature.getTypeParameters()).map(async (parameter) => {
                // require the parameter's declaration
                const symbol = await parameter.getSymbol();
                const handle = symbol?.declarations[0];
                const generic = handle === undefined ? undefined : await this.node(handle);
                if (!generic || !isTypeParameterDeclaration(generic)) {
                    throw new BuildError(
                        "INSPECTION_FAILED",
                        "missing generic parameter declaration",
                    );
                }

                return await this.parameterType(generic);
            }),
        );
    }

    /** Describe a signature's parameters, leaving their texts to the signature text. */
    async #parameters(signature: Signature): Promise<SignatureDescription["parameters"]> {
        // read the parameters' declarations and types
        const parameters = await signature.getParameters();
        const [nodes, types] = await Promise.all([
            Promise.all(parameters.map((parameter) => this.nodes(parameter))),
            this.#typesOf(parameters),
        ]);

        // describe argument omission independently of undefined in its type
        return await Promise.all(
            parameters.map(async (parameter, index) => {
                // require a resolved parameter type
                const declaration = aligned(nodes, index).find(isParameterDeclaration);
                const type = aligned(types, index);
                if (type.isErrorType()) {
                    throw new BuildError("INSPECTION_FAILED", "cannot inspect an unresolved type");
                }

                return {
                    name: declaration?.name.getText() ?? parameter.name,
                    references: await this.#references(type),
                    isOptional: Boolean(
                        declaration?.questionToken ||
                        declaration?.initializer ||
                        parameter.flags & SymbolFlags.Optional,
                    ),
                    isRest: Boolean(declaration?.dotDotDotToken),
                };
            }),
        );
    }

    /** Read a generic parameter from its source declaration. */
    async parameterType(node: Node): Promise<SourceDeclaration["typeParameters"][number]> {
        if (!isTypeParameterDeclaration(node)) {
            throw new BuildError("INSPECTION_FAILED", "expected a type parameter");
        }

        // describe the parameter and each authored type expression
        const result: SourceDeclaration["typeParameters"][number] = {
            name: node.name.getText(),
        };
        for (const key of ["constraint", "default"] as const) {
            const expression = key === "default" ? node.defaultType : node.constraint;
            if (!expression) {
                continue;
            }

            // resolve the authored constraint or default
            const type = await this.project.checker.getTypeFromTypeNode(expression);
            if (!type) {
                throw new BuildError("INSPECTION_FAILED", `missing generic ${key}: ${result.name}`);
            }

            result[key] = await this.type(type, node);
        }

        return result;
    }

    /** Read documentation through the compiler's symbol model. */
    async documentation(symbol: TypeScriptSymbol): Promise<SymbolDescription["documentation"]> {
        return {
            text: await this.project.checker.getDocumentationCommentOfSymbol(symbol),
            tags: (await this.project.checker.getJsDocTagsOfSymbol(symbol)).map(({ name, text }) =>
                text === undefined ? { name } : { name, text },
            ),
        };
    }

    /** Resolve the source declarations of a symbol. */
    nodes(symbol: TypeScriptSymbol): Promise<Node[]> {
        return Promise.all(symbol.declarations.map((handle) => this.node(handle)));
    }

    /** Resolve a symbol's declarations in the package's modules, leaving dependency augmentations. */
    async #authored(symbol: TypeScriptSymbol): Promise<Node[]> {
        const nodes = await this.nodes(symbol);

        return nodes.filter((node) => isAuthored(this.root, node.getSourceFile().fileName));
    }

    /** Name a node's range by its file's package-relative path, read once per file. */
    range(node: Node): SourceRange {
        // name each file once, since ranges cover every expression
        const file = node.getSourceFile().fileName;
        let path = this.#paths.get(file);
        if (path === undefined) {
            path = relativePath(this.root, file);
            this.#paths.set(file, path);
        }

        return { file: path, start: node.getStart(), end: node.getEnd() };
    }

    /** Resolve a declaration, fetching its source file once however many walks resolve it together. */
    async node(handle: NodeHandle): Promise<Node> {
        // fetch the source file once, after which the compiler client serves its nodes
        let source = this.#sources.get(handle.path);
        if (source === undefined) {
            source = this.project.program.getSourceFile(handle.path);
            this.#sources.set(handle.path, source);
        }
        await source;

        // require the declaration
        const node = await handle.resolve(this.project);
        if (!node) {
            throw new BuildError("INSPECTION_FAILED", `missing declaration: ${handle.path}`);
        }

        return node;
    }
}

/** Read the name a symbol's declaration writes, a private member's without the compiler's per-program prefix, which differs between compilations. */
export function declaredName(symbol: TypeScriptSymbol): string {
    return PRIVATE_NAME.exec(symbol.name)?.[1] ?? symbol.name;
}

/** Read a declaration's node list property, requiring each node to pass a guard. */
function nodeList<Item extends Node>(
    list: unknown,
    guard: (node: Node) => node is Item,
): readonly Item[] {
    if (list === undefined) {
        return [];
    } else if (!isNodeList(list) || !list.every(guard)) {
        throw new BuildError("INSPECTION_FAILED", "a declaration has an unexpected node list");
    }

    return list;
}

/** Report whether a value is a list of syntax nodes. */
function isNodeList(value: unknown): value is readonly Node[] {
    return (
        Array.isArray(value) &&
        value.every((item: unknown) => typeof item === "object" && item !== null && "kind" in item)
    );
}
