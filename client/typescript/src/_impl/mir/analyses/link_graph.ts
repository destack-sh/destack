import type { BitSet } from "../../../_generated/core/bitset.js";
import { BitSetImpl } from "../../core/bitset.js";
import type { CallComponentGraph, LinkEdge, LinkNode, LinkGraph } from "../../../_generated/mir/analyses/link_graph.js";
import type { Symbol } from "../../../_generated/mir/tree/symbol.js";

export const CallComponentGraphImpl = {
    /** Return the component id of a symbol by dense id. */
    componentId(graph: CallComponentGraph, symbol: number): number {
        return graph.component[symbol];
    },

    /** Return whether a symbol's component contains a cycle. */
    isRecursive(graph: CallComponentGraph, symbol: number): boolean {
        return BitSetImpl.contains(graph.recursive as BitSet, graph.component[symbol]);
    },

    /** Return the number of components. */
    len(graph: CallComponentGraph): number {
        return graph.recursive.length;
    },

    /** Return whether there are no components. */
    isEmpty(graph: CallComponentGraph): boolean {
        return graph.recursive.length === 0;
    },
};

export const LinkGraphImpl = {
    /** Return one symbol node. */
    node(graph: LinkGraph, symbol: Symbol): LinkNode | undefined {
        return getSymbolValue(graph.nodes, symbol);
    },

    /** Return all defined symbols and their nodes. */
    definedSymbols(graph: LinkGraph): ReadonlyArray<readonly [Symbol, LinkNode]> {
        return [...graph.nodes.entries()];
    },

    /** Return the outgoing references from one source symbol. */
    edgesFor(graph: LinkGraph, source: Symbol): ReadonlyArray<LinkEdge> {
        return getSymbolValue(graph.edges, source) ?? [];
    },
};

/** Return one map value keyed by MIR symbol. */
function getSymbolValue<Value>(
    map: ReadonlyMap<Symbol, Value>,
    key: Symbol,
): Value | undefined {
    for (const [candidate, value] of map) {
        // compare structural symbols because JS maps key objects by identity
        if (candidate.value === key.value) {
            return value;
        }
    }

    return undefined;
}
