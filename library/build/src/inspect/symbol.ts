import { BuildError } from "../error/index.ts";
import {
    NodeBuilderFlags,
    type Project,
    type Signature,
    SignatureKind,
    type Symbol as TypeScriptSymbol,
    SymbolFlags,
    type Type,
} from "typescript/unstable/async";
import {
    isIdentifier,
    isParameterDeclaration,
    isTypeAliasDeclaration,
    isTypeParameterDeclaration,
    type Node,
    SyntaxKind,
} from "typescript/unstable/ast";
import type {
    DeclarationDescription,
    SignatureDescription,
    SymbolDescription,
    SymbolReference,
    TypeDescription,
} from "@destack/package/code";
import type { SourceRange } from "@destack/package/source";

/** Render complete type expressions through TypeScript's TypeFormatFlags.NoTruncation. */
const NO_TRUNCATION = 1;

/** Named declarations retained as references instead of expanded structural types. */
const REFERENCE_FLAGS =
    SymbolFlags.Class |
    SymbolFlags.Interface |
    SymbolFlags.TypeAlias |
    SymbolFlags.Enum |
    SymbolFlags.Function |
    SymbolFlags.ValueModule;

/** Extract public API descriptions within one compiler snapshot. */
export class SymbolInspector {
    /** The compiler project being inspected. */
    readonly project: Project;
    /** Resolve named declarations and enqueue local dependencies. */
    readonly reference: (symbol: TypeScriptSymbol) => Promise<SymbolReference>;
    /** Convert a source node into a package-relative range. */
    readonly range: (node: Node) => SourceRange;

    /** Associate compiler results with package declarations and source files. */
    constructor(
        project: Project,
        reference: (symbol: TypeScriptSymbol) => Promise<SymbolReference>,
        range: (node: Node) => SourceRange,
    ) {
        this.project = project;
        this.reference = reference;
        this.range = range;
    }

    /** Describe a symbol's type and value namespaces independently. */
    async describe(symbol: TypeScriptSymbol, name: string): Promise<SymbolDescription> {
        // collect merged declarations and their documentation
        const nodes = await this.nodes(symbol);
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
        let declared: Type | undefined;
        if (symbol.flags & SymbolFlags.Type) {
            declared = await this.project.checker.getDeclaredTypeOfSymbol(symbol);
            result.declaredType = await this.type(declared, nodes[0]);
            result.typeSignatures = await this.signatures(declared, nodes[0]);

            // retain index signatures
            for (const index of await this.project.checker.getIndexInfosOfType(declared)) {
                result.indexes.push({
                    key: await this.type(index.keyType, nodes[0]),
                    value: await this.type(index.valueType, nodes[0]),
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

        // describe the value namespace
        if (symbol.flags & SymbolFlags.Value) {
            const type = await this.project.checker.getTypeOfSymbol(symbol);
            if (!type) {
                throw new BuildError("INSPECTION_FAILED", `Missing value type: ${symbol.name}`);
            }

            result.valueType = await this.type(type, nodes[0]);
            result.signatures = await this.signatures(type, nodes[0]);
        }

        // retain declared members, including accessors and class static members
        const members = [...(await symbol.getMembers()).values()];
        const exports = [...(await symbol.getExports()).values()];
        if (symbol.flags & SymbolFlags.Module) {
            for (const exported of exports) {
                const original =
                    exported.flags & SymbolFlags.Alias
                        ? await this.project.checker.getAliasedSymbol(exported)
                        : exported;
                result.exports.push({
                    name: exported.name,
                    symbol: await this.reference(original),
                });
            }
        } else {
            members.push(...exports);
        }

        // collect members declared in object type aliases
        if (
            declared &&
            nodes.some(
                (node) => isTypeAliasDeclaration(node) && node.type.kind === SyntaxKind.TypeLiteral,
            )
        ) {
            const owner = await declared.getSymbol();
            if (owner && !(owner.flags & (SymbolFlags.Class | SymbolFlags.Interface))) {
                members.push(...(await owner.getMembers()).values());
            }
        }

        // describe each declared member once
        const seen = new Set<number>();
        for (const member of members) {
            if (seen.has(member.id) || !member.declarations.length) {
                continue;
            }

            seen.add(member.id);
            if (
                member.flags &
                (SymbolFlags.TypeParameter | SymbolFlags.Signature | SymbolFlags.Constructor)
            ) {
                continue;
            }

            result.members.push(await this.member(member, symbol));
        }

        return result;
    }

    /** Describe a member's declarations, type, overloads, and documentation. */
    async member(
        symbol: TypeScriptSymbol,
        parent: TypeScriptSymbol,
    ): Promise<SymbolDescription["members"][number]> {
        // resolve aliases before selecting the member type
        const declarations = await this.nodes(symbol);
        const original =
            symbol.flags & SymbolFlags.Alias
                ? await this.project.checker.getAliasedSymbol(symbol)
                : symbol;

        // inspect the original declaration in its type or value namespace
        const type =
            original.flags & SymbolFlags.Type
                ? await this.project.checker.getDeclaredTypeOfSymbol(original)
                : await this.project.checker.getTypeOfSymbolAtLocation(original, declarations[0]);
        if (type.isErrorType()) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Cannot inspect member: ${parent.name}.${symbol.name} (${symbol.flags}).`,
            );
        }

        return {
            name: symbol.name,
            declarations: await Promise.all(declarations.map((node) => this.declaration(node))),
            isOptional: Boolean(symbol.flags & SymbolFlags.Optional),
            type: await this.type(type, declarations[0]),
            signatures: await this.signatures(type, declarations[0]),
            documentation: await this.documentation(symbol),
        };
    }

    /** Describe one source declaration independently of merged symbols. */
    async declaration(node: Node): Promise<DeclarationDescription> {
        // retain source ranges independently for merged declarations
        const result: DeclarationDescription = {
            kind: SyntaxKind[node.kind],
            source: this.range(node),
            modifiers: [],
            typeParameters: [],
            heritage: [],
        };

        // retain modifiers and generic parameters on their original declarations
        if ("modifiers" in node && Array.isArray(node.modifiers)) {
            result.modifiers = node.modifiers.map((modifier) => modifier.getText());
        }

        // preserve generic constraints and defaults
        if ("typeParameters" in node && Array.isArray(node.typeParameters)) {
            for (const parameter of node.typeParameters) {
                result.typeParameters.push(await this.parameterType(parameter));
            }
        }

        // retain each declared base type and its named references
        if ("heritageClauses" in node && Array.isArray(node.heritageClauses)) {
            for (const clause of node.heritageClauses) {
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
        while (pending.length) {
            const current = pending.pop()!;
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
            const original =
                symbol.flags & SymbolFlags.Alias
                    ? await this.project.checker.getAliasedSymbol(symbol)
                    : symbol;
            if (!(original.flags & REFERENCE_FLAGS)) {
                continue;
            }

            const reference = await this.reference(original);
            result.set(JSON.stringify(reference), reference);
        }

        return [...result.values()];
    }

    /** Describe an expression and the named declarations needed to understand it. */
    async type(type: Type, location: Node): Promise<TypeDescription> {
        if (type.isErrorType()) {
            throw new BuildError("INSPECTION_FAILED", "Cannot inspect an unresolved type.");
        }

        // track visited types and named references
        const references = new Map<string, SymbolReference>();
        const visited = new Set<number>();
        const pending = [type];

        // walk type expressions once and stop expansion at named declarations
        while (pending.length) {
            const current = pending.pop()!;
            if (visited.has(current.id)) {
                continue;
            }
            visited.add(current.id);

            // retain named declarations without expanding their members
            const symbol = (await current.getAliasSymbol()) ?? (await current.getSymbol());
            const isNamed =
                symbol && symbol.declarations.length > 0 && Boolean(symbol.flags & REFERENCE_FLAGS);
            if (isNamed) {
                const reference = await this.reference(symbol);
                references.set(JSON.stringify(reference), reference);
            }

            // inspect generic arguments even when the enclosing type is named
            pending.push(...(await current.getAliasTypeArguments()));
            if (current.isTypeReference()) {
                pending.push(...(await this.project.checker.getTypeArguments(current)));
            }

            if (isNamed) {
                continue;
            }

            pending.push(...(await this.expand(current)));
        }

        return {
            text: await this.project.checker.typeToString(type, location, NO_TRUNCATION),
            references: [...references.entries()]
                .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
                .map(([, reference]) => reference),
        };
    }

    /** Collect constituent types from an unnamed type expression. */
    private async expand(type: Type): Promise<Type[]> {
        const result: Type[] = [];

        // expand composite expressions
        if (type.isUnionType() || type.isIntersectionType() || type.isTemplateLiteralType()) {
            result.push(...(await type.getTypes()));
        }
        // inspect the object and key of an indexed access
        else if (type.isIndexedAccessType()) {
            result.push(await type.getObjectType(), await type.getIndexType());
        }
        // follow type operators to their target
        else if (type.isIndexType() || type.isStringMappingType()) {
            result.push(await type.getTarget());
        }
        // inspect both conditional branches
        else if (type.isConditionalType()) {
            result.push(
                await type.getCheckType(),
                await type.getExtendsType(),
                await type.getTrueType(),
                await type.getFalseType(),
            );
        }
        // retain substitution constraints
        else if (type.isSubstitutionType()) {
            result.push(await type.getBaseType(), await type.getConstraint());
        }
        // inspect structural members and callable signatures
        else if (type.isObjectType()) {
            for (const property of await this.project.checker.getPropertiesOfType(type)) {
                const propertyType = await this.project.checker.getTypeOfSymbol(property);
                if (!propertyType) {
                    throw new BuildError(
                        "INSPECTION_FAILED",
                        `Missing property type: ${property.name}`,
                    );
                }

                result.push(propertyType);
            }

            // inspect return and parameter types for each overload
            for (const kind of [SignatureKind.Call, SignatureKind.Construct]) {
                for (const signature of await this.project.checker.getSignaturesOfType(
                    type,
                    kind,
                )) {
                    // require a resolved return type
                    const returns = await this.project.checker.getReturnTypeOfSignature(signature);
                    if (!returns) {
                        throw new BuildError("INSPECTION_FAILED", "Missing signature return type.");
                    }
                    result.push(returns);

                    // retain each parameter type
                    for (const parameter of await signature.getParameters()) {
                        const parameterType = await this.project.checker.getTypeOfSymbol(parameter);
                        if (!parameterType) {
                            throw new BuildError(
                                "INSPECTION_FAILED",
                                `Missing parameter type: ${parameter.name}`,
                            );
                        }

                        result.push(parameterType);
                    }
                }
            }
        }

        return result;
    }

    /** Describe each callable and constructable overload in compiler order. */
    async signatures(type: Type, location: Node): Promise<SignatureDescription[]> {
        const result: SignatureDescription[] = [];
        for (const kind of [SignatureKind.Call, SignatureKind.Construct]) {
            for (const signature of await this.project.checker.getSignaturesOfType(type, kind)) {
                result.push(await this.signature(signature, location));
            }
        }

        return result;
    }

    /** Describe a callable signature, including receiver and rest parameters. */
    async signature(signature: Signature, location: Node): Promise<SignatureDescription> {
        // ask the compiler to render the complete signature
        const declaration = (await signature.declaration?.resolve(this.project)) ?? location;
        const node = await this.project.checker.signatureToSignatureDeclaration(
            signature,
            signature.isConstruct ? SyntaxKind.ConstructSignature : SyntaxKind.CallSignature,
            declaration,
            NodeBuilderFlags.NoTruncation | NodeBuilderFlags.UseAliasDefinedOutsideCurrentScope,
        );
        const returns = await this.project.checker.getReturnTypeOfSignature(signature);
        if (!node || !returns) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Cannot describe a callable signature: ${declaration?.getText()}`,
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

        // preserve generic constraints and defaults from their original declarations
        for (const parameter of await signature.getTypeParameters()) {
            const symbol = await parameter.getSymbol();
            const declaration = await symbol?.declarations[0]?.resolve(this.project);
            if (!declaration || !isTypeParameterDeclaration(declaration)) {
                throw new BuildError("INSPECTION_FAILED", "Missing generic parameter declaration.");
            }
            result.typeParameters.push(await this.parameterType(declaration));
        }

        // describe argument omission independently of undefined in its type
        for (const parameter of await signature.getParameters()) {
            const declarations = await this.nodes(parameter);
            const parameterDeclaration = declarations.find(isParameterDeclaration);
            const type = await this.project.checker.getTypeOfSymbol(parameter);
            if (!type) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `Missing parameter type: ${parameter.name}`,
                );
            }

            result.parameters.push({
                name: parameterDeclaration?.name.getText() ?? parameter.name,
                type: await this.type(type, parameterDeclaration ?? declaration),
                isOptional: Boolean(
                    parameterDeclaration?.questionToken ||
                    parameterDeclaration?.initializer ||
                    parameter.flags & SymbolFlags.Optional,
                ),
                isRest: Boolean(parameterDeclaration?.dotDotDotToken),
            });
        }

        // retain an explicit this parameter
        const receiver = await signature.getThisParameter();
        if (receiver) {
            const type = await this.project.checker.getTypeOfSymbol(receiver);
            if (!type) {
                throw new BuildError("INSPECTION_FAILED", "Missing receiver type.");
            }
            result.receiver = await this.type(type, declaration);
        }

        return result;
    }

    /** Read a generic parameter from its source declaration. */
    async parameterType(node: Node): Promise<DeclarationDescription["typeParameters"][number]> {
        if (!isTypeParameterDeclaration(node)) {
            throw new BuildError("INSPECTION_FAILED", "Expected a type parameter.");
        }

        // describe the parameter and each authored type expression
        const result: DeclarationDescription["typeParameters"][number] = {
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
                throw new BuildError("INSPECTION_FAILED", `Missing generic ${key}: ${result.name}`);
            }

            result[key] = await this.type(type, node);
        }

        return result;
    }

    /** Read documentation through the compiler's symbol model. */
    async documentation(symbol: TypeScriptSymbol): Promise<SymbolDescription["documentation"]> {
        return {
            text: await this.project.checker.getDocumentationCommentOfSymbol(symbol),
            tags: [...(await this.project.checker.getJsDocTagsOfSymbol(symbol))],
        };
    }

    /** Resolve the source declarations of a symbol. */
    nodes(symbol: TypeScriptSymbol): Promise<Node[]> {
        return Promise.all(
            symbol.declarations.map(async (handle) => {
                const node = await handle.resolve(this.project);
                if (!node) {
                    throw new BuildError(
                        "INSPECTION_FAILED",
                        `Missing declaration: ${symbol.name}`,
                    );
                }

                return node;
            }),
        );
    }
}
