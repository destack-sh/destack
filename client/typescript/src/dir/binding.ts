import type { ModuleId } from "../_generated/source/file/model/module.js";
import type { BindingSegment } from "../_generated/dir/table/binding.js";
import type { LocalScope, LocalScopeId, Scope } from "../_generated/dir/symbol/scope.js";
import type { LocalSymbolId, Symbol } from "../_generated/dir/symbol/symbol.js";
import type { GlobalNodeIdAny } from "../_generated/dir/tree/node.js";

const LOCAL_SCOPE_MARK_END = 0xffffffff;

/** Cumulative lexical scopes and symbols for one DIR module. */
export class BindingTable {
    /** The module id of this binding table. */
    readonly moduleId: ModuleId;

    /** The ordered binding table segments. */
    private readonly segments: readonly BindingSegment[];

    /** Create one binding table from ordered segments. */
    private constructor(segments: readonly BindingSegment[]) {
        if (segments.length === 0) {
            throw new Error("binding table needs at least one segment");
        }

        const moduleId = segments[0].moduleId;
        for (const segment of segments) {
            if (!moduleIdEquals(segment.moduleId, moduleId)) {
                throw new Error("binding table segment belongs to a different module");
            }
        }

        this.moduleId = moduleId;
        this.segments = segments;
    }

    /** Create one binding table from ordered segments. */
    static fromSegments(...segments: readonly BindingSegment[]): BindingTable {
        return new BindingTable(segments);
    }

    /** Return the number of visible symbols. */
    symbolCount(): number {
        const segment = this.segments.at(-1);

        return segment === undefined ? 0 : segmentSymbolCount(segment);
    }

    /** Return the number of visible scopes. */
    scopeCount(): number {
        const segment = this.segments.at(-1);

        return segment === undefined ? 0 : segmentScopeCount(segment);
    }

    /** Return one visible symbol. */
    symbol(symbolId: LocalSymbolId): Symbol {
        const symbol = this.symbolMaybe(symbolId);
        if (symbol === undefined) {
            throw new Error(`DIR symbol ${symbolId.id} is not visible`);
        }

        return symbol;
    }

    /** Return one visible symbol when present. */
    symbolMaybe(symbolId: LocalSymbolId): Symbol | undefined {
        for (let index = this.segments.length - 1; index >= 0; index -= 1) {
            const symbol = segmentSymbolMaybe(this.segments[index], symbolId);
            if (symbol !== undefined) {
                return symbol;
            }
        }

        return undefined;
    }

    /** Return one visible scope. */
    scope(scopeId: LocalScopeId): Scope {
        const scope = this.scopeMaybe(scopeId);
        if (scope === undefined) {
            throw new Error(`DIR scope ${scopeId} is not visible`);
        }

        return scope;
    }

    /** Return one visible scope when present. */
    scopeMaybe(scopeId: LocalScopeId): Scope | undefined {
        for (let index = this.segments.length - 1; index >= 0; index -= 1) {
            const scope = segmentScopeMaybe(this.segments[index], scopeId);
            if (scope !== undefined) {
                return scope;
            }
        }

        return undefined;
    }

    /** Find the symbol declared by one node. */
    declarationSymbol(declaration: GlobalNodeIdAny): LocalSymbolId | undefined {
        for (let index = this.segments.length - 1; index >= 0; index -= 1) {
            const symbol = globalNodeMapGet(
                this.segments[index].symbolByDeclaration,
                declaration,
            );
            if (symbol !== undefined) {
                return symbol;
            }
        }

        return undefined;
    }

    /** Find the implicit receiver symbol bound for one member node. */
    implicitReceiverSymbol(owner: GlobalNodeIdAny): LocalSymbolId | undefined {
        for (let index = this.segments.length - 1; index >= 0; index -= 1) {
            const symbol = globalNodeMapGet(
                this.segments[index].implicitReceiverByNode,
                owner,
            );
            if (symbol !== undefined) {
                return symbol;
            }
        }

        return undefined;
    }

    /** Return the lexical scope attached to one global node. */
    scopeForNode(node: GlobalNodeIdAny): LocalScope | undefined {
        for (let index = this.segments.length - 1; index >= 0; index -= 1) {
            const scope = globalNodeMapGet(this.segments[index].scopeByNode, node);
            if (scope !== undefined) {
                return scope;
            }
        }

        return undefined;
    }

    /** Return the owned scope for one visible symbol. */
    scopeForOwner(owner: LocalSymbolId): LocalScope | undefined {
        for (let index = this.segments.length - 1; index >= 0; index -= 1) {
            const scopeId = localSymbolMapGet(this.segments[index].scopeByOwner, owner);
            if (scopeId !== undefined) {
                return { id: scopeId, mark: LOCAL_SCOPE_MARK_END };
            }
        }

        return undefined;
    }
}

/** Return the number of symbols visible after one segment. */
function segmentSymbolCount(segment: BindingSegment): number {
    return segment.firstSymbolId + segment.symbols.length;
}

/** Return the number of scopes visible after one segment. */
function segmentScopeCount(segment: BindingSegment): number {
    return segment.firstScopeId + segment.scopes.length;
}

/** Return one symbol owned or replaced by a segment. */
function segmentSymbolMaybe(segment: BindingSegment, symbolId: LocalSymbolId): Symbol | undefined {
    const replaced = localSymbolMapGet(segment.replacedSymbolById, symbolId);
    if (replaced !== undefined) {
        return replaced;
    }

    if (symbolId.id < segment.firstSymbolId || symbolId.id >= segmentSymbolCount(segment)) {
        return undefined;
    }

    return segment.symbols[symbolId.id - segment.firstSymbolId];
}

/** Return one scope owned or replaced by a segment. */
function segmentScopeMaybe(segment: BindingSegment, scopeId: LocalScopeId): Scope | undefined {
    const replaced = localScopeMapGet(segment.replacedScopeById, scopeId);
    if (replaced !== undefined) {
        return replaced;
    }

    if (scopeId < segment.firstScopeId || scopeId >= segmentScopeCount(segment)) {
        return undefined;
    }

    return segment.scopes[scopeId - segment.firstScopeId];
}

/** Return one value keyed by a local symbol id. */
function localSymbolMapGet<T>(
    map: ReadonlyMap<LocalSymbolId, T>,
    key: LocalSymbolId,
): T | undefined {
    for (const [entry, value] of map) {
        if (entry.id === key.id) {
            return value;
        }
    }

    return undefined;
}

/** Return one value keyed by a local scope id. */
function localScopeMapGet<T>(
    map: ReadonlyMap<LocalScopeId, T>,
    key: LocalScopeId,
): T | undefined {
    for (const [entry, value] of map) {
        if (entry === key) {
            return value;
        }
    }

    return undefined;
}

/** Return one value keyed by a global node id. */
function globalNodeMapGet<T>(
    map: ReadonlyMap<GlobalNodeIdAny, T>,
    key: GlobalNodeIdAny,
): T | undefined {
    for (const [entry, value] of map) {
        if (globalNodeIdEquals(entry, key)) {
            return value;
        }
    }

    return undefined;
}

/** Return whether two global node ids are equal. */
function globalNodeIdEquals(left: GlobalNodeIdAny, right: GlobalNodeIdAny): boolean {
    return moduleIdEquals(left.moduleId, right.moduleId)
        && left.localId.id === right.localId.id
        && left.localId.ty === right.localId.ty;
}

/** Return whether two module ids are equal. */
function moduleIdEquals(left: ModuleId, right: ModuleId): boolean {
    return left.packageId === right.packageId
        && left.moduleKey === right.moduleKey;
}
