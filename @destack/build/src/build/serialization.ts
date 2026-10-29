import { Text } from "@destack/schema";
import { BuildError } from "../error/index.ts";

/** Serialize an inspection value without implicit conversions or discarded values. */
export function stringifyInspection(value: unknown, indent = 0): string {
    // refuse each non-JSON value as the serializer reaches it, by its path from the root
    const paths = new Map<object, string>();
    const replacer = function (this: object, key: string, raw: unknown): unknown {
        // check the value at its path, and remember an object's path for its own values
        const path = key === "" ? "$" : `${paths.get(this) ?? "$"}[${JSON.stringify(key)}]`;
        checkValue(this, key, raw, path);
        if (typeof raw === "object" && raw !== null) {
            paths.set(raw, path);
        }

        return raw;
    };

    // refuse cyclic values, which the serializer reports as a type error
    try {
        return JSON.stringify(value, replacer, indent);
    } catch (error) {
        if (error instanceof TypeError) {
            throw BuildError.of("INSPECTION_FAILED", `cyclic inspection value: ${error.message}`);
        }
        throw error;
    }
}

/** Require a JSON value, allowing an absent optional object property. */
function checkValue(holder: object, key: string, raw: unknown, path: string): void {
    // accept JSON scalars without conversion, and an absent object property
    const isElement = Array.isArray(holder);
    if (raw === null || typeof raw === "string" || typeof raw === "boolean") {
        return;
    } else if (typeof raw === "number" && Number.isFinite(raw)) {
        return;
    } else if (raw === undefined && !isElement && key !== "") {
        return;
    }
    // refuse other scalars, holes and undefined elements
    else if (typeof raw !== "object") {
        throw BuildError.of("INSPECTION_FAILED", `non-JSON inspection value at ${path}`);
    }

    // refuse objects with serialization behavior, and symbol keys
    const prototype: unknown = Object.getPrototypeOf(raw);
    const plain: unknown = Array.isArray(raw) ? Array.prototype : Object.prototype;
    if ((prototype !== plain && prototype !== null) || Object.hasOwn(raw, "toJSON")) {
        throw BuildError.of("INSPECTION_FAILED", `non-JSON inspection object at ${path}`);
    } else if (Object.getOwnPropertySymbols(raw).length > 0) {
        throw BuildError.of("INSPECTION_FAILED", `non-JSON inspection property at ${path}`);
    }
}

/** Order two files by package path, equal paths as equal. */
export function comparePath(
    left: { readonly path: string },
    right: { readonly path: string },
): number {
    return Text.compare(left.path, right.path);
}
