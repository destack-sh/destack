import { defineSchema, schema } from "@destack/schema";
import { SourceRange } from "../source/location.ts";
import { TypeDescription } from "./type.ts";
import { SymbolReference } from "./reference.ts";

/** A function's explicit throws, calls, and enclosing exception handlers. */
export const ErrorDescription = defineSchema(
    schema.object({
        /** The function declaration, or module for top-level code. */
        source: SourceRange,
        /** Explicit throw expressions, including non-Error values. */
        throws: schema.array(
            schema.object({
                /** The throw statement. */
                source: SourceRange,
                /** The compiler-resolved thrown type. */
                type: TypeDescription,
                /** Enclosing catches, ordered from nearest to outermost. */
                catches: schema.array(SourceRange),
            }),
        ),
        /** Calls whose failures may propagate; a target does not prove its failure behavior. */
        calls: schema.array(
            schema.object({
                /** The call expression. */
                source: SourceRange,
                /** The resolved callee, absent for unresolved or indirect calls. */
                target: SymbolReference.optional(),
                /** Whether this call is directly awaited. */
                isAwaited: schema.boolean(),
                /** Enclosing catches for synchronous failures, ordered nearest first. */
                catches: schema.array(SourceRange),
            }),
        ),
        /** Catch bodies, including their explicit rethrows and translated errors. */
        catches: schema.array(SourceRange),
        /** Finally bodies, whose failures or returns can replace an earlier outcome. */
        finally: schema.array(SourceRange),
        /** Unresolved behavior that prevents a complete escaping-error calculation. */
        unknowns: schema.array(
            schema.object({
                /** The expression requiring runtime knowledge. */
                source: SourceRange,
                /** The missing analysis. */
                reason: schema.enum(["call", "property", "iteration", "await", "implicit"]),
            }),
        ),
    }),
);

/** A function's error relationships. */
export type ErrorDescription = schema.Infer<typeof ErrorDescription>;
