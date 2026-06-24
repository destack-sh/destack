import type { ScopeKind } from "../../../_generated/dir/symbol/scope.js";
import type { SymbolKind, SymbolRole } from "../../../_generated/dir/symbol/symbol.js";
import type { Declaration } from "../../../_generated/dir/tree/declaration.js";

export const DeclarationImpl = {
    /** Return whether the declaration is ambient. */
    isAmbient(declaration: Declaration): boolean {
        // global declarations expose their own ambient flag
        if (declaration.kind === "global") {
            return declaration.global.isAmbient;
        }

        // type declarations expose their own ambient flag
        else if (declaration.kind === "type") {
            return declaration.type.isAmbient;
        }

        // struct declarations expose their own ambient flag
        else if (declaration.kind === "struct") {
            return declaration.struct.isAmbient;
        }

        // class declarations expose their own ambient flag
        else if (declaration.kind === "class") {
            return declaration.class.isAmbient;
        }

        // enum declarations expose their own ambient flag
        else if (declaration.kind === "enum") {
            return declaration.enum.isAmbient;
        }

        // interface declarations expose their own ambient flag
        else if (declaration.kind === "interface") {
            return declaration.interface.isAmbient;
        }

        // extension declarations expose their own ambient flag
        else if (declaration.kind === "extension") {
            return declaration.extension.isAmbient;
        }

        // function declarations expose their own ambient flag
        else if (declaration.kind === "function") {
            return declaration.function.isAmbient;
        }
        // module and error declarations are not ambient
        else {
            return false;
        }
    },

    /** Return whether the declaration is abstract. */
    isAbstract(declaration: Declaration): boolean {
        // class declarations store abstractness directly
        if (declaration.kind === "class") {
            return declaration.class.isAbstract;
        }

        // functions store abstractness on their signature
        else if (declaration.kind === "function") {
            return declaration.function.signature.isAbstract;
        }
        // all other declaration forms are concrete
        else {
            return false;
        }
    },

    /** Return the declaration symbol kind. */
    symbolKind(declaration: Declaration): SymbolKind | undefined {
        // global and module declarations do not introduce regular symbols
        if (declaration.kind === "global" || declaration.kind === "module") {
            return undefined;
        }

        // nominal aliases use a distinct symbol kind
        else if (declaration.kind === "type") {
            return declaration.type.isNominal ? "newtype" : "typeAlias";
        }

        // nominal interfaces use a distinct symbol kind
        else if (declaration.kind === "interface") {
            return declaration.interface.isNominal ? "newtypeInterface" : "interface";
        }

        // functions use the stable function symbol kind
        else if (declaration.kind === "function") {
            return "function";
        }
        // other declaration variants map directly to symbol kinds
        else {
            return declaration.kind;
        }
    },

    /** Return the declaration symbol role. */
    symbolRole(declaration: Declaration): SymbolRole | undefined {
        // global and module declarations do not introduce regular symbols
        if (declaration.kind === "global" || declaration.kind === "module") {
            return undefined;
        }

        // functions and type aliases are leaf items
        else if (declaration.kind === "function" || declaration.kind === "type") {
            return "item";
        }
        // aggregate declaration forms own namespace members
        else {
            return "namespace";
        }
    },

    /** Return the scope kind opened by the declaration. */
    symbolScopeKind(declaration: Declaration): ScopeKind | undefined {
        // functions open function scopes
        if (declaration.kind === "function") {
            return "function";
        }

        // namespace declarations open namespace scopes
        else if (DeclarationImpl.symbolRole(declaration) === "namespace") {
            return "namespace";
        }
        // leaf declarations do not open scopes
        else {
            return undefined;
        }
    },
};
