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
        throw new BuildError("INSPECTION_FAILED", `non-JSON inspection value at ${path}`);
    }

    // reject objects with serialization behavior and cyclic references
    const isArray = Array.isArray(value);
    const prototype: unknown = Object.getPrototypeOf(value);
    const plain: unknown = isArray ? Array.prototype : Object.prototype;
    if (prototype !== plain && prototype !== null) {
        throw new BuildError("INSPECTION_FAILED", `non-JSON inspection object at ${path}`);
    }
    if (ancestors.has(value)) {
        throw new BuildError("INSPECTION_FAILED", `cyclic inspection value at ${path}`);
    }
    ancestors.add(value);

    // examine descriptors without invoking getters or toJSON methods
    const properties = Object.getOwnPropertyDescriptors(value);
    for (const [key, property] of Object.entries(properties)) {
        checkProperty(key, property, isArray ? value : undefined, path, ancestors);
    }

    // reject symbol keys, leaving non-enumerable object metadata outside the JSON document
    for (const key of Object.getOwnPropertySymbols(value)) {
        if (isArray || Object.prototype.propertyIsEnumerable.call(value, key)) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `non-JSON inspection property at ${path}[${JSON.stringify(String(key))}]`,
            );
        }
    }

    // reject array holes, which JSON would replace with null
    if (isArray && Object.keys(properties).length !== value.length + 1) {
        throw new BuildError("INSPECTION_FAILED", `sparse inspection array at ${path}`);
    }
    ancestors.delete(value);
}

/** Check one own property of an inspection object or array, skipping absent optional fields. */
function checkProperty(
    key: string,
    property: PropertyDescriptor,
    array: readonly unknown[] | undefined,
    path: string,
    ancestors: Set<object>,
): void {
    // skip an array's length and an object's non-enumerable metadata
    const location = `${path}[${JSON.stringify(key)}]`;
    if (array !== undefined && key === "length") {
        return;
    } else if (array === undefined && property.enumerable !== true && key !== "toJSON") {
        return;
    }

    // require a data property at an array index
    if (!("value" in property)) {
        throw new BuildError("INSPECTION_FAILED", `non-JSON inspection property at ${location}`);
    }
    if (array !== undefined && (!/^(0|[1-9][0-9]*)$/u.test(key) || Number(key) >= array.length)) {
        throw new BuildError("INSPECTION_FAILED", `non-JSON array property at ${location}`);
    }

    // check a present value
    if (array !== undefined || property.value !== undefined) {
        checkValue(property.value, location, ancestors);
    }
}

/** Order two strings by UTF-16 code units, independent of the host locale. */
export function compareText(left: string, right: string): number {
    // order the lesser string first
    if (left < right) {
        return -1;
    }
    // order the greater string last
    else if (left > right) {
        return 1;
    }
    // keep equal strings in place
    else {
        return 0;
    }
}

/** Order two files by package path, equal paths as equal. */
export function comparePath(
    left: { readonly path: string },
    right: { readonly path: string },
): number {
    return compareText(left.path, right.path);
}
