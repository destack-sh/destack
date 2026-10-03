import type { ESTree, Rule } from "@oxlint/plugins";

/** The tag of an implementation building a value whose type its signatures map from the input, with the invariant that makes it hold. */
const CONSTRUCT = /@construct\s+\S/u;

/** A statement or class member as the rule reads it: its name, whether it has a body, and its return type. */
interface Declared {
    /** The node the rule reports. */
    readonly node: ESTree.Node;
    /** The declared name, absent for nodes that declare no function. */
    readonly name: string | undefined;
    /** Whether the declaration is a signature without a body. */
    readonly isSignature: boolean;
    /** The source text of the declared return type, absent without an annotation. */
    readonly returns: string | undefined;
}

/** Reject an implementation under overload signatures, which callers see instead of what it returns, unless it states the construction it performs. */
export const noOverloadCast: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "type an implementation itself instead of under overloads" },
        messages: {
            cast: "'{{name}}' declares overloads over its implementation, which cast what callers see; type the implementation itself, or tag a mapped construction with `@construct <invariant>`",
        },
    },
    create(context) {
        /** Read a return type annotation's source text, absent without one. */
        function returnText(annotation: ESTree.Node | null | undefined): string | undefined {
            return annotation === null || annotation === undefined
                ? undefined
                : context.sourceCode.getText(annotation);
        }

        /** Report each implementation following signatures of its name. */
        function check(declarations: readonly Declared[]): void {
            // collect the return types of the signatures before each implementation
            let name: string | undefined;
            let returns: (string | undefined)[] = [];
            for (const declared of declarations) {
                // collect a signature of the current name, starting a new run for another name
                if (declared.isSignature) {
                    returns =
                        declared.name === name
                            ? [...returns, declared.returns]
                            : [declared.returns];
                    name = declared.name;
                }
                // report an implementation whose signatures return other than it does, unless it states its construction
                else if (
                    declared.name !== undefined &&
                    declared.name === name &&
                    returns.some((text) => text === undefined || text !== declared.returns)
                ) {
                    const documentation = context.sourceCode
                        .getCommentsBefore(declared.node)
                        .at(-1);
                    if (documentation === undefined || !CONSTRUCT.test(documentation.value)) {
                        context.report({ node: declared.node, messageId: "cast", data: { name } });
                    }
                    name = undefined;
                    returns = [];
                }
                // end the run at any other declaration
                else {
                    name = undefined;
                    returns = [];
                }
            }
        }

        /** Read a statement as a function declaration or signature of its name, or as any other statement. */
        function statementOf(
            statement: ESTree.Statement | ESTree.Directive | ESTree.ModuleDeclaration,
        ): Declared {
            const declaration =
                statement.type === "ExportNamedDeclaration" ||
                statement.type === "ExportDefaultDeclaration"
                    ? statement.declaration
                    : statement;
            if (
                declaration?.type === "TSDeclareFunction" ||
                declaration?.type === "FunctionDeclaration"
            ) {
                return {
                    node: statement,
                    name: declaration.id?.name,
                    isSignature: declaration.type === "TSDeclareFunction",
                    returns: returnText(declaration.returnType),
                };
            }

            return { node: statement, name: undefined, isSignature: false, returns: undefined };
        }

        return {
            Program(node) {
                check(node.body.map(statementOf));
            },
            BlockStatement(node) {
                check(node.body.map(statementOf));
            },
            TSModuleBlock(node) {
                check(node.body.map(statementOf));
            },
            ClassBody(node) {
                check(
                    node.body.map((member) => {
                        if (
                            member.type === "MethodDefinition" ||
                            member.type === "TSAbstractMethodDefinition"
                        ) {
                            const key = member.key;
                            const name =
                                key.type === "Identifier"
                                    ? `${member.static ? "static " : ""}${key.name}`
                                    : key.type === "PrivateIdentifier"
                                      ? `#${key.name}`
                                      : undefined;

                            return {
                                node: member,
                                name,
                                isSignature: member.value.type === "TSEmptyBodyFunctionExpression",
                                returns: returnText(member.value.returnType),
                            };
                        }

                        return {
                            node: member,
                            name: undefined,
                            isSignature: false,
                            returns: undefined,
                        };
                    }),
                );
            },
        };
    },
};
