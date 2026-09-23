import { BuildError } from "../error/index.ts";

/** Serialize an inspection value without implicit conversions or discarded values. */
export function stringifyInspection(value: unknown, indent = 0): string {
    checkValue(value, "$", new Set());

    return JSON.stringify(value, null, indent);
}

/** Require JSON values, allowing absent optional object properties. */
function checkValue(value: unknown, path: string, ancestors: Set<object>): void {
    // accept JSON scalars without conversion
    if (value === null || typeof value === "string" || typeof value === "boolean") {
        return;
    }
    if (typeof value === "number" && Number.isFinite(value)) {
        return;
    }
    if (typeof value !== "object" || value === null) {
        throw new BuildError("INSPECTION_FAILED", `Non-JSON inspection value at ${path}.`);
    }

    // reject objects with serialization behavior and cyclic references
    const isArray = Array.isArray(value);
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== (isArray ? Array.prototype : Object.prototype) && prototype !== null) {
        throw new BuildError("INSPECTION_FAILED", `Non-JSON inspection object at ${path}.`);
    }
    if (ancestors.has(value)) {
        throw new BuildError("INSPECTION_FAILED", `Cyclic inspection value at ${path}.`);
    }
    ancestors.add(value);

    // examine descriptors without invoking getters or toJSON methods
    const properties = Object.getOwnPropertyDescriptors(value);
    for (const key of Reflect.ownKeys(properties)) {
        if (isArray && key === "length") {
            continue;
        }
        const property = properties[key as string];
        const location = `${path}[${JSON.stringify(String(key))}]`;
        // leave non-enumerable object metadata outside the JSON document
        if (!isArray && !property.enumerable && key !== "toJSON") {
            continue;
        }
        if (typeof key !== "string" || !("value" in property)) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Non-JSON inspection property at ${location}.`,
            );
        }
        if (isArray && (!/^(0|[1-9][0-9]*)$/.test(key) || Number(key) >= value.length)) {
            throw new BuildError("INSPECTION_FAILED", `Non-JSON array property at ${location}.`);
        }
        if (!isArray && property.value === undefined) {
            continue;
        }
        checkValue(property.value, location, ancestors);
    }

    // reject array holes, which JSON would replace with null
    if (isArray && Object.keys(properties).length !== value.length + 1) {
        throw new BuildError("INSPECTION_FAILED", `Sparse inspection array at ${path}.`);
    }
    ancestors.delete(value);
}
