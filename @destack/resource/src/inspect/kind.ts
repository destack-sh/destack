import { type JsonSchema, toJsonSchema } from "@destack/schema";
import type { ResourceKind } from "../declare/kind.ts";

/** Describe a resource kind by its name and the JSON Schemas of its specification and desired state. */
export function describeResourceKind(kind: ResourceKind): {
    readonly name: string;
    readonly spec: JsonSchema;
    readonly state?: JsonSchema;
} {
    return {
        name: kind.name,
        spec: toJsonSchema(kind.spec),
        ...(kind.state === undefined ? {} : { state: toJsonSchema(kind.state) }),
    };
}
