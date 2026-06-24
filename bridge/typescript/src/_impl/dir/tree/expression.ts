import type { StaticKey } from "../../../_generated/dir/symbol/key.js";
import type { Expression } from "../../../_generated/dir/tree/expression.js";

export const ExpressionImpl = {
    /** Return a static key for simple scalar literals. */
    staticKey(expression: Expression): StaticKey | undefined {
        if (expression.kind !== "scalarLiteral") {
            return undefined;
        }

        const literal = expression.scalar_literal;

        // non-negative integer literals map to numeric static keys
        if (literal.kind === "integer" && literal.integer >= 0n) {
            const index = Number(literal.integer);

            // unsafe integer keys cannot roundtrip through generated numbers
            if (Number.isSafeInteger(index)) {
                return { kind: "index", index };
            }
        }

        // string literals map to named static keys
        else if (literal.kind === "string") {
            return { kind: "name", name: literal.string };
        }
        // other literals do not have static keys
        else {
            return undefined;
        }

        return undefined;
    },

    /** Return whether the expression directly names a value. */
    isReference(expression: Expression): boolean {
        return expression.kind === "identifier" || expression.kind === "this" || expression.kind === "super";
    },
};
