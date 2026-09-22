import { AccessError } from "../error/index.ts";
import type { Grant } from "../grant/grant.ts";
import type { ShareToken } from "../grant/token.ts";
import { type AccessObject, type ObjectReference, objectKey } from "./object.ts";

/** A synchronous authoritative read view that remains stable during one decision. */
export interface AccessView {
    /** Change this revision whenever any visible authorization record changes. */
    readonly revision: string | number;
    /** Resolve one object without fetching unrelated application state. */
    object(reference: ObjectReference): AccessObject | undefined;
    /** Read explicit grants indexed by the complete object reference. */
    grants(reference: ObjectReference): readonly Grant[];
    /** Resolve current bearer-token state within its authority scope. */
    token(scope: string, id: string): ShareToken | undefined;
}

/** An immutable indexed snapshot for offline inspection and reproducible decisions. */
export class AccessSnapshot implements AccessView {
    /** Empty grant collections share one immutable array. */
    static readonly #empty: readonly Grant[] = Object.freeze([]);
    /** Protected records indexed by their complete object references. */
    readonly #objects = new Map<string, AccessObject>();
    /** Sharing grants grouped by their protected object. */
    readonly #grants = new Map<string, Grant[]>();
    /** Bearer credentials indexed by scope and identifier. */
    readonly #tokens = new Map<string, ShareToken>();

    /** Copy and index one snapshot; reuse it for any number of decisions. */
    constructor(
        readonly revision: string | number,
        objects: readonly AccessObject[],
        grants: readonly Grant[],
        tokens: readonly ShareToken[] = [],
    ) {
        // freeze copied values so a caller cannot change the snapshot through a returned record
        for (const object of structuredClone(objects)) {
            const key = objectKey(object.reference);
            if (this.#objects.has(key)) {
                throw new AccessError("CONFLICT", `duplicate access object: ${key}`);
            }
            this.#objects.set(key, freeze(object));
        }

        // group grants by object without expanding ownership or inherited relationships
        for (const grant of structuredClone(grants)) {
            const key = objectKey(grant.object);
            const entries = this.#grants.get(key) ?? [];
            entries.push(freeze(grant));
            this.#grants.set(key, entries);
        }
        for (const grants of this.#grants.values()) {
            Object.freeze(grants);
        }

        // index current bearer-token state by authority and identifier
        for (const token of structuredClone(tokens)) {
            const key = JSON.stringify([token.scope, token.id]);
            if (this.#tokens.has(key)) {
                throw new AccessError("CONFLICT", `duplicate share token: ${key}`);
            }
            this.#tokens.set(key, freeze(token));
        }
    }

    /** Resolve an immutable application record. */
    object(reference: ObjectReference): AccessObject | undefined {
        return this.#objects.get(objectKey(reference));
    }

    /** Resolve immutable grants without scanning other objects. */
    grants(reference: ObjectReference): readonly Grant[] {
        return this.#grants.get(objectKey(reference)) ?? AccessSnapshot.#empty;
    }

    /** Resolve current token state from this snapshot. */
    token(scope: string, id: string): ShareToken | undefined {
        return this.#tokens.get(JSON.stringify([scope, id]));
    }
}

/** Freeze records recursively once at snapshot construction. */
function freeze<Value>(value: Value): Value {
    if (value !== null && typeof value === "object") {
        for (const child of Object.values(value)) {
            freeze(child);
        }
        Object.freeze(value);
    }

    return value;
}
