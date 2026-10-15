import type { JsonObject } from "@destack/schema";
import type { StackDeclaration } from "../declare/stack.ts";

/** Describe a declared stack for the package manifest. */
export function describeStack(stack: StackDeclaration): JsonObject {
    return stack.definition;
}
