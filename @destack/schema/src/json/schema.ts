import { z } from "zod";
import { requireDeclarable } from "../validate/declarable.ts";

/** The JSON Schema Draft 2020-12 description for inspection and external tooling. */
export type JsonSchema = z.core.JSONSchema.JSONSchema;

/** Describe a schema using JSON Schema Draft 2020-12. */
export function toJsonSchema(schema: z.ZodType): JsonSchema {
    requireDeclarable(schema);

    return z.toJSONSchema(schema, {
        target: "draft-2020-12",
        unrepresentable: "throw",
        io: "input",
    });
}

/** The reference prefix of a schema's own definitions. */
const DEFINITIONS = "#/$defs/";

/** Build a validator from a JSON Schema Draft 2020-12 description, following every local reference. */
export function fromJsonSchema(description: JsonSchema): z.ZodType {
    return z.fromJSONSchema(hoisted(description), { defaultTarget: "draft-2020-12" });
}

/** Move the target of each local reference outside `$defs` into `$defs`, such as `#/components/schemas/Pet`, so every reference names a definition. */
function hoisted(description: JsonSchema): JsonSchema {
    // leave a boolean schema as it is
    if (typeof description !== "object") {
        return description;
    }

    // rewrite each local reference to a definition, hoisting its target once
    const definitions: Record<string, unknown> = {};
    const rewrite = (node: unknown): unknown => {
        // rewrite the entries of lists and objects, leaving other values
        if (Array.isArray(node)) {
            return node.map(rewrite);
        } else if (typeof node !== "object" || node === null) {
            return node;
        }

        // rewrite a local reference outside the definitions, hoisting its target once
        const rewritten = Object.fromEntries(
            Object.entries(node).map(([key, value]) => [key, rewrite(value)]),
        );
        const reference = rewritten["$ref"];
        if (
            typeof reference !== "string" ||
            !reference.startsWith("#/") ||
            reference.startsWith(DEFINITIONS)
        ) {
            return rewritten;
        }
        const key = reference.slice(2).replaceAll("/", ".");
        if (!(key in definitions)) {
            definitions[key] = true;
            definitions[key] = rewrite(targetOf(description, reference));
        }

        return { ...rewritten, $ref: `${DEFINITIONS}${key}` };
    };
    const root = rewrite(description);
    const own = description.$defs === undefined ? {} : rewrite(description.$defs);

    return Object.assign({}, description, root, { $defs: Object.assign({}, own, definitions) });
}

/** Read the schema a local JSON Pointer reference (RFC 6901) points at, refusing one that does not resolve. */
function targetOf(root: object, reference: string): unknown {
    // walk the pointer's unescaped segments from the root
    let found: unknown = root;
    for (const segment of reference.slice(2).split("/")) {
        const key = segment.replaceAll("~1", "/").replaceAll("~0", "~");
        found = typeof found === "object" && found !== null ? Reflect.get(found, key) : undefined;
    }
    if (found === undefined) {
        throw new TypeError(`schema reference ${reference} does not resolve`);
    }

    return found;
}
