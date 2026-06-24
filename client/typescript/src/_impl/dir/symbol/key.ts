import type { StaticKey, SymbolKey } from "../../../_generated/dir/symbol/key.js";
import type { GlobalSymbolId } from "../../../_generated/dir/symbol/symbol.js";
import type { StringId } from "../../../_generated/core/string.js";

export const StaticKeyImpl = {
    /** Return whether two static keys refer to the same key. */
    matches(left: StaticKey, right: StaticKey): boolean {
        // different key variants cannot match
        if (left.kind !== right.kind) {
            return false;
        }

        // compare string keys by interned string value
        if (left.kind === "name" && right.kind === "name") {
            return stringMatches(left.name, right.name);
        }

        // compare index keys by numeric index
        else if (left.kind === "index" && right.kind === "index") {
            return left.index === right.index;
        }
        // compare symbol keys by their symbol payload
        else {
            return left.kind === "symbol" && right.kind === "symbol" && symbolMatches(left.symbol, right.symbol);
        }
    },

    /** Return whether the key names a string-like member. */
    isStringLike(key: StaticKey): boolean {
        return key.kind === "name";
    },

    /** Return whether the key names a number-like member. */
    isNumberLike(key: StaticKey): boolean {
        return key.kind === "index";
    },

    /** Return whether the key names a symbol-like member. */
    isSymbolLike(key: StaticKey): boolean {
        return key.kind === "symbol";
    },
};

/** Return whether two symbol keys refer to the same symbol. */
function symbolMatches(left: SymbolKey, right: SymbolKey): boolean {
    // different symbol key variants cannot match
    if (left.kind !== right.kind) {
        return false;
    }

    // unique symbols are scoped by module and local symbol ids
    if (left.kind === "unique" && right.kind === "unique") {
        return globalSymbolMatches(left.unique, right.unique);
    }

    // registry symbols are scoped by registry string
    else {
        return left.kind === "registry" && right.kind === "registry" && stringMatches(left.registry, right.registry);
    }
}

/** Return whether two global symbols refer to the same symbol. */
function globalSymbolMatches(left: GlobalSymbolId, right: GlobalSymbolId): boolean {
    const isSamePackage = left.moduleId.packageId === right.moduleId.packageId;
    const isSameModule = left.moduleId.moduleKey === right.moduleId.moduleKey;
    const isSameLocal = left.localId.id === right.localId.id;

    return isSamePackage && isSameModule && isSameLocal;
}

/** Return whether two interned string ids refer to the same string. */
function stringMatches(left: StringId, right: StringId): boolean {
    return left === right;
}
