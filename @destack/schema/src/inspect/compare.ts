import { canonicalize } from "../json/json.ts";
import type { JsonSchema } from "../json/schema.ts";

/** How the values a schema accepts changed, from the same values to unrelated ones. */
export type SchemaComparison = "same" | "wider" | "narrower" | "incompatible";

/** The JSON value types a schema may accept. */
const TYPES = ["string", "number", "integer", "boolean", "null", "array", "object"] as const;

/** The keywords that describe a schema without constraining its values. */
const ANNOTATIONS = new Set([
    "$schema",
    "$id",
    "$comment",
    "$defs",
    "id",
    "title",
    "description",
    "deprecated",
    "examples",
    "default",
    "readOnly",
    "writeOnly",
]);

/** The keywords the comparison reads, beside annotations. */
const KEYWORDS = new Set([
    "$ref",
    "type",
    "const",
    "enum",
    "anyOf",
    "oneOf",
    "allOf",
    "minLength",
    "maxLength",
    "pattern",
    "format",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "items",
    "prefixItems",
    "minItems",
    "maxItems",
    "properties",
    "required",
    "additionalProperties",
    "propertyNames",
    "minProperties",
    "maxProperties",
]);

/** Compare two JSON Schema Draft 2020-12 descriptions by the values each accepts. */
export function compareJsonSchemas(before: JsonSchema, after: JsonSchema): SchemaComparison {
    const isWider = new Inclusion(after, before).includes(after, before);
    const isNarrower = new Inclusion(before, after).includes(before, after);

    return isWider && isNarrower
        ? "same"
        : isWider
          ? "wider"
          : isNarrower
            ? "narrower"
            : "incompatible";
}

/** One schema node: a keyword object or a boolean schema. */
type Node = boolean | JsonSchema;

/** A proof that one schema accepts every value another accepts, answering no when it cannot prove it. */
class Inclusion {
    /** The document the including schema's references resolve in. */
    readonly #outerRoot: JsonSchema;
    /** The document the included schema's references resolve in. */
    readonly #innerRoot: JsonSchema;
    /** The node pairs under proof, assumed included when a recursive schema reaches them again. */
    readonly #assumed = new Map<Node, Set<Node>>();

    /** Prove inclusions between nodes of two documents. */
    constructor(outerRoot: JsonSchema, innerRoot: JsonSchema) {
        this.#outerRoot = outerRoot;
        this.#innerRoot = innerRoot;
    }

    /** Report whether the outer node accepts every value the inner node accepts. */
    includes(outer: Node, inner: Node): boolean {
        // follow references and assume pairs already under proof on this path
        const outerNode = resolve(outer, this.#outerRoot);
        const innerNode = resolve(inner, this.#innerRoot);
        const pairs = this.#assumed.get(outerNode) ?? new Set<Node>();
        if (pairs.has(innerNode)) {
            return true;
        }

        // prove the pair under its own assumption and drop the assumption after
        this.#assumed.set(outerNode, pairs.add(innerNode));
        const isIncluded = this.#includesNode(outerNode, innerNode);
        pairs.delete(innerNode);

        return isIncluded;
    }

    /** Report whether one resolved node accepts every value another accepts. */
    #includesNode(outerNode: Node, innerNode: Node): boolean {
        // settle boolean schemas
        if (innerNode === false || isAny(outerNode)) {
            return true;
        } else if (outerNode === false) {
            return false;
        }
        const outerKeywords = outerNode === true ? {} : outerNode;
        const innerKeywords = innerNode === true ? {} : innerNode;

        return this.#includesKeywords(outerKeywords, innerKeywords);
    }

    /** Report whether outer keywords accept every value inner keywords accept. */
    #includesKeywords(outer: JsonSchema, inner: JsonSchema): boolean {
        // split conjunctions and alternatives off either side
        const { allOf: outerAll, ...outerRest } = outer;
        const innerBranches = inner.anyOf ?? inner.oneOf;
        const outerBranches = outer.anyOf ?? outer.oneOf;
        const listed = inner.const === undefined ? inner.enum : [inner.const];

        // require the inner values in every part of an outer conjunction
        if (outerAll !== undefined) {
            return (
                this.includes(outerRest, inner) &&
                outerAll.every((part) => this.includes(part, inner))
            );
        }
        // check each listed inner value
        else if (listed !== undefined) {
            return listed.every((value) => this.#accepts(outer, value));
        }
        // require every inner alternative
        else if (innerBranches !== undefined) {
            const { anyOf, oneOf, ...rest } = inner;

            return innerBranches.every((branch) => this.includes(outer, { allOf: [rest, branch] }));
        }
        // split inner type lists into one alternative per type
        else if (Array.isArray(inner.type) && inner.type.length > 1) {
            return inner.type.every((type) => this.includes(outer, { ...inner, type }));
        }
        // narrow through any part of an inner conjunction, such as an inner alternative nested in another
        else if (inner.allOf !== undefined && this.#includesPart(outer, inner)) {
            return true;
        }
        // find one outer alternative including every inner value
        else if (outerBranches !== undefined) {
            const { anyOf, oneOf, ...rest } = outer;

            return (
                this.includes(rest, inner) &&
                outerBranches.some((branch) => this.includes(branch, inner))
            );
        }
        // refuse an inner conjunction no part of which the outer includes
        else if (inner.allOf !== undefined) {
            return false;
        }

        return this.#includesTyped(outer, inner);
    }

    /** Report whether outer keywords accept every value of some part of an inner conjunction. */
    #includesPart(outer: JsonSchema, inner: JsonSchema): boolean {
        const { allOf = [], ...rest } = inner;

        return [rest, ...allOf].some((part) => this.includes(outer, part));
    }

    /** Report whether single-alternative outer keywords accept every value of each inner type. */
    #includesTyped(outer: JsonSchema, inner: JsonSchema): boolean {
        // accept an identical schema and refuse unreadable outer keywords or unmatched value lists
        const isListed = outer.const !== undefined || outer.enum !== undefined;
        const isUnreadable = Object.keys(outer).some((keyword) => !isKnown(keyword));
        if ((isListed || isUnreadable) && equals(outer, inner)) {
            return true;
        } else if (isListed || isUnreadable) {
            return false;
        }

        // require each inner type among the outer types, with its constraints
        const outerTypes = typesOf(outer);

        return [...typesOf(inner)].every((type) => {
            const isAllowed =
                outerTypes.has(type) || (type === "integer" && outerTypes.has("number"));
            if (!isAllowed) {
                return false;
            } else if (type === "string") {
                return includesString(outer, inner);
            } else if (type === "number" || type === "integer") {
                return includesNumber(outer, inner, type, outerTypes);
            } else if (type === "array") {
                return this.#includesArray(outer, inner);
            } else if (type === "object") {
                return this.#includesObject(outer, inner);
            }

            return true;
        });
    }

    /** Report whether outer array keywords accept every inner array. */
    #includesArray(outer: JsonSchema, inner: JsonSchema): boolean {
        // require the inner length within the outer bounds
        const isLength =
            (outer.minItems ?? 0) <= (inner.minItems ?? 0) &&
            innerMaxItems(inner) <= (outer.maxItems ?? Infinity);
        if (!isLength) {
            return false;
        }

        // compare every position either side fixes and the rest
        const outerPrefix = outer.prefixItems ?? [];
        const innerPrefix = inner.prefixItems ?? [];
        const positions = Math.max(outerPrefix.length, innerPrefix.length);
        for (let index = 0; index < positions; index += 1) {
            const outerItem = outerPrefix[index] ?? itemsOf(outer);
            const innerItem = innerPrefix[index] ?? itemsOf(inner);
            if (!this.includes(outerItem, innerItem)) {
                return false;
            }
        }

        return this.includes(itemsOf(outer), itemsOf(inner));
    }

    /** Report whether outer object keywords accept every inner object. */
    #includesObject(outer: JsonSchema, inner: JsonSchema): boolean {
        // require the outer required properties and property counts
        const innerRequired = new Set(inner.required ?? []);
        const isCounted =
            (outer.required ?? []).every((name) => innerRequired.has(name)) &&
            (outer.minProperties ?? 0) <= (inner.minProperties ?? 0) &&
            (inner.maxProperties ?? Infinity) <= (outer.maxProperties ?? Infinity);
        if (!isCounted) {
            return false;
        }

        // compare every property either side declares
        const outerProperties = outer.properties ?? {};
        const innerProperties = inner.properties ?? {};
        const outerRest = outer.additionalProperties ?? true;
        const innerRest = inner.additionalProperties ?? true;
        const names = new Set([...Object.keys(outerProperties), ...Object.keys(innerProperties)]);
        for (const name of names) {
            const outerProperty = outerProperties[name] ?? outerRest;
            const innerProperty = innerProperties[name] ?? innerRest;
            const isNamed =
                outerProperties[name] !== undefined ||
                outer.propertyNames === undefined ||
                this.#accepts(outer.propertyNames, name);
            if (!isNamed || !this.includes(outerProperty, innerProperty)) {
                return false;
            }
        }

        // compare the undeclared properties and their names
        const outerNames = outer.propertyNames ?? true;
        const innerNames = inner.propertyNames ?? { type: "string" };

        return (
            innerRest === false ||
            (this.includes(outerRest, innerRest) && this.includes(outerNames, innerNames))
        );
    }

    /** Report whether an outer node accepts a JSON value. */
    #accepts(outer: Node, value: unknown): boolean {
        // follow references and settle boolean schemas
        const node = resolve(outer, this.#outerRoot);
        if (typeof node === "boolean") {
            return node;
        }

        // require every conjunct, one alternative and a listed value
        const branches = node.anyOf ?? node.oneOf;
        const isListed =
            (node.const === undefined || equals(node.const, value)) &&
            (node.enum === undefined || node.enum.some((entry) => equals(entry, value)));
        const isCombined =
            (node.allOf ?? []).every((part) => this.#accepts(part, value)) &&
            (branches === undefined || branches.some((branch) => this.#accepts(branch, value)));
        if (!isListed || !isCombined) {
            return false;
        }

        // refuse keywords the comparison cannot read and values of other types
        const type = typeOf(value);
        const types = typesOf(node);
        const isTyped = types.has(type) || (type === "integer" && types.has("number"));
        if (Object.keys(node).some((keyword) => !isKnown(keyword)) || !isTyped) {
            return false;
        }

        // check the value's own constraints
        if (typeof value === "string") {
            return acceptsString(node, value);
        } else if (typeof value === "number") {
            return acceptsNumber(node, value);
        } else if (Array.isArray(value)) {
            return this.#acceptsArray(node, value);
        } else if (typeof value === "object" && value !== null) {
            return this.#acceptsObject(node, value);
        }

        return true;
    }

    /** Report whether array keywords accept an array. */
    #acceptsArray(node: JsonSchema, value: readonly unknown[]): boolean {
        const prefix = node.prefixItems ?? [];
        const isLength =
            value.length >= (node.minItems ?? 0) && value.length <= (node.maxItems ?? Infinity);

        return (
            isLength &&
            value.every((item, index) => this.#accepts(prefix[index] ?? itemsOf(node), item))
        );
    }

    /** Report whether object keywords accept an object. */
    #acceptsObject(node: JsonSchema, value: object): boolean {
        // require the listed properties and the property count
        const entries = Object.entries(value);
        const properties = node.properties ?? {};
        const isCounted =
            (node.required ?? []).every((name) => name in value) &&
            entries.length >= (node.minProperties ?? 0) &&
            entries.length <= (node.maxProperties ?? Infinity);

        return (
            isCounted &&
            entries.every(
                ([name, field]) =>
                    (properties[name] !== undefined ||
                        this.#accepts(node.propertyNames ?? true, name)) &&
                    this.#accepts(properties[name] ?? node.additionalProperties ?? true, field),
            )
        );
    }
}

/** Follow a node's local references within its document. */
function resolve(node: Node, root: JsonSchema): Node {
    // keep nodes without a reference
    if (typeof node === "boolean" || node.$ref === undefined) {
        return node;
    }

    // find the referenced definition
    const { $ref: reference, ...rest } = node;
    const match = /^#(?:\/\$defs\/(.+))?$/u.exec(reference);
    const name = match?.[1];
    const found = name === undefined ? root : root.$defs?.[decodePointer(name)];
    if (match === null || found === undefined) {
        throw new TypeError(`unresolved schema reference: ${reference}`);
    }

    // keep keywords beside the reference as a conjunction
    const isBare = Object.keys(rest).every((keyword) => ANNOTATIONS.has(keyword));

    return isBare ? resolve(found, root) : { allOf: [found, rest] };
}

/** Decode one JSON Pointer segment. */
function decodePointer(segment: string): string {
    return segment.replaceAll("~1", "/").replaceAll("~0", "~");
}

/** Report whether a node accepts every value. */
function isAny(node: Node): boolean {
    return (
        node === true || (node !== false && Object.keys(node).every((key) => ANNOTATIONS.has(key)))
    );
}

/** Report whether the comparison reads or ignores a keyword. */
function isKnown(keyword: string): boolean {
    return KEYWORDS.has(keyword) || ANNOTATIONS.has(keyword);
}

/** Read the types a node accepts, every type when it lists none. */
function typesOf(node: JsonSchema): Set<string> {
    // take the listed types
    if (node.type !== undefined) {
        return new Set(typeof node.type === "string" ? [node.type] : node.type);
    }

    // accept every type, integers within numbers
    return new Set(TYPES.filter((type) => type !== "integer"));
}

/** Read a JSON value's type, integers apart from other numbers. */
function typeOf(value: unknown): string {
    if (value === null) {
        return "null";
    } else if (Array.isArray(value)) {
        return "array";
    } else if (typeof value === "number") {
        return Number.isInteger(value) ? "integer" : "number";
    }

    return typeof value;
}

/** Report whether outer string keywords accept every inner string. */
function includesString(outer: JsonSchema, inner: JsonSchema): boolean {
    return (
        (outer.minLength ?? 0) <= (inner.minLength ?? 0) &&
        (inner.maxLength ?? Infinity) <= (outer.maxLength ?? Infinity) &&
        (outer.pattern === undefined || outer.pattern === inner.pattern) &&
        (outer.format === undefined || outer.format === inner.format)
    );
}

/** Report whether outer number keywords accept every inner number of a type. */
function includesNumber(
    outer: JsonSchema,
    inner: JsonSchema,
    type: "number" | "integer",
    outerTypes: ReadonlySet<string>,
): boolean {
    // compare the bounds, an exclusive bound tighter than an equal inclusive one
    const isLower = includesBound(lowerBound(outer), lowerBound(inner), -1);
    const isUpper = includesBound(upperBound(outer), upperBound(inner), 1);

    // require inner numbers to be multiples of the outer step
    const isIntegral = outerTypes.has("number") || type === "integer";
    const isMultiple =
        outer.multipleOf === undefined ||
        (inner.multipleOf !== undefined && Number.isInteger(inner.multipleOf / outer.multipleOf));

    return isLower && isUpper && isIntegral && isMultiple;
}

/** Report whether string keywords accept a string. */
function acceptsString(node: JsonSchema, value: string): boolean {
    const length = Array.from(value).length;

    return (
        length >= (node.minLength ?? 0) &&
        length <= (node.maxLength ?? Infinity) &&
        (node.pattern === undefined || new RegExp(node.pattern, "u").test(value)) &&
        node.format === undefined
    );
}

/** Report whether number keywords accept a number. */
function acceptsNumber(node: JsonSchema, value: number): boolean {
    return (
        includesBound(lowerBound(node), [value, false], -1) &&
        includesBound(upperBound(node), [value, false], 1) &&
        (node.multipleOf === undefined || Number.isInteger(value / node.multipleOf))
    );
}

/** Read a node's lower number bound and whether it is exclusive. */
function lowerBound(node: JsonSchema): readonly [number, boolean] {
    const inclusive = node.minimum ?? -Infinity;
    const exclusive = boundOf(node.exclusiveMinimum) ?? -Infinity;

    return exclusive >= inclusive ? [exclusive, exclusive !== -Infinity] : [inclusive, false];
}

/** Read a node's upper number bound and whether it is exclusive. */
function upperBound(node: JsonSchema): readonly [number, boolean] {
    const inclusive = node.maximum ?? Infinity;
    const exclusive = boundOf(node.exclusiveMaximum) ?? Infinity;

    return exclusive <= inclusive ? [exclusive, exclusive !== Infinity] : [inclusive, false];
}

/** Read the schema of the array positions past the prefix, every value when absent. */
function itemsOf(node: JsonSchema): Node {
    if (Array.isArray(node.items)) {
        throw new TypeError("draft 4 tuple items are unsupported");
    }

    return node.items ?? true;
}

/** Read an exclusive number bound, refusing the draft 4 boolean form. */
function boundOf(bound: number | boolean | undefined): number | undefined {
    if (typeof bound === "boolean") {
        throw new TypeError("draft 4 exclusive bounds are unsupported");
    }

    return bound;
}

/** Read an inner array's greatest length, bounded by its fixed positions when it forbids the rest. */
function innerMaxItems(inner: JsonSchema): number {
    const fixed = inner.items === false ? (inner.prefixItems ?? []).length : Infinity;

    return Math.min(inner.maxItems ?? Infinity, fixed);
}

/** Report whether an outer bound admits everything an inner bound admits, on the side a direction points to. */
function includesBound(
    outer: readonly [number, boolean],
    inner: readonly [number, boolean],
    direction: -1 | 1,
): boolean {
    const [outerValue, isOuterExclusive] = outer;
    const [innerValue, isInnerExclusive] = inner;

    // admit an inner bound strictly inside, or at an equal bound unless only the outer one excludes it
    return (
        (innerValue - outerValue) * direction < 0 ||
        (innerValue === outerValue && (!isOuterExclusive || isInnerExclusive))
    );
}

/** Compare two JSON values structurally. */
function equals(left: unknown, right: unknown): boolean {
    return canonicalize(left) === canonicalize(right);
}
