import type { StaticKey } from "../../../_generated/dir/symbol/key.js";
import type { SymbolKind, SymbolSpace } from "../../../_generated/dir/symbol/symbol.js";
import type { GenericParameter, Parameter } from "../../../_generated/dir/tree/argument.js";
import type { LocalNodeId } from "../../../_generated/dir/tree/node.js";

export const GenericParameterImpl = {
    /** Return the key declared by the parameter. */
    symbolKey(parameter: GenericParameter): StaticKey | undefined {
        // error parameters do not declare names
        if (parameter.kind === "error") {
            return undefined;
        }

        return { kind: "name", name: parameter.name };
    },

    /** Return the symbol space declared by the parameter. */
    symbolSpace(parameter: GenericParameter): SymbolSpace | undefined {
        // type parameters bind in type space
        if (parameter.kind === "type" || parameter.kind === "variadicType") {
            return "type";
        }

        // value parameters bind in value space
        else if (parameter.kind === "value" || parameter.kind === "variadicValue") {
            return "value";
        }
        // error parameters do not bind in a symbol space
        else {
            return undefined;
        }
    },

    /** Return the symbol kind declared by the parameter. */
    symbolKind(parameter: GenericParameter): SymbolKind | undefined {
        // type parameters declare generic type symbols
        if (parameter.kind === "type" || parameter.kind === "variadicType") {
            return "genericTypeParameter";
        }

        // value parameters declare generic value symbols
        else if (parameter.kind === "value" || parameter.kind === "variadicValue") {
            return "genericValueParameter";
        }
        // error parameters do not declare symbols
        else {
            return undefined;
        }
    },
};

export const ParameterImpl = {
    /** Return the key declared by the parameter. */
    symbolKey(parameter: Parameter): StaticKey | undefined {
        // only named parameter forms bind a parameter name
        if (parameter.kind === "named" || parameter.kind === "variadicNamed") {
            return { kind: "name", name: parameter.name };
        }

        return undefined;
    },

    /** Return the parameter default value node. */
    defaultValue(parameter: Parameter): LocalNodeId | undefined {
        // only concrete parameter forms can carry defaults
        if (parameter.kind === "named" || parameter.kind === "pattern") {
            return parameter.default;
        }

        return undefined;
    },
};
