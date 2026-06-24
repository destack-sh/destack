import type { StringId } from "../../../_generated/core/string.js";
import type { StaticKey } from "../../../_generated/dir/symbol/key.js";
import type { Key, Name as NameValue } from "../../../_generated/dir/tree/key.js";

export const NameImpl = {
    /** Return the static key for a property name. */
    staticKey(name: NameValue): StaticKey {
        return staticKey(name);
    },
};

export const KeyImpl = {
    /** Return the direct static key for a non-private key. */
    directStaticKey(key: Key): StaticKey | undefined {
        if (key.kind !== "name") {
            return undefined;
        }

        return staticKey(key.name);
    },

    /** Return the private name for a private key. */
    privateName(key: Key): StringId | undefined {
        return key.kind === "private" ? key.private : undefined;
    },
};

/** Return the static key for a property name. */
function staticKey(name: NameValue): StaticKey {
    // numeric names map to index keys
    if (name.kind === "index") {
        return { kind: "index", index: name.index };
    }

    // identifiers map to named keys
    else if (name.kind === "identifier") {
        return { kind: "name", name: name.identifier };
    }
    // string names map to named keys
    else {
        return { kind: "name", name: name.string };
    }
}
