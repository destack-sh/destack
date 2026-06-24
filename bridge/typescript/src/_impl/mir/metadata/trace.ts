import type {
    TraceId,
    TraceMap,
    TraceTable,
} from "../../../_generated/mir/metadata/trace.js";

export const TraceMapImpl = {
    /** Return the empty trace map. */
    empty(): TraceMap {
        return { kind: "empty" };
    },

    /** Return whether this map can reach heap references. */
    hasReference(map: TraceMap): boolean {
        return TraceMapImpl.hasLocalReference(map) || TraceMapImpl.hasSharedReference(map);
    },

    /** Return whether this map can reach local heap references. */
    hasLocalReference(map: TraceMap): boolean {
        if (map.kind === "empty") {
            return false;
        }

        // fixed maps carry direct local offsets
        if (map.kind === "fixed") {
            return map.localOffsets.length > 0;
        }

        // nested maps delegate to their child map
        if (map.kind === "nested") {
            return TraceMapImpl.hasLocalReference(map.map);
        }

        // composite maps aggregate child maps
        if (map.kind === "composite") {
            return map.maps.some(TraceMapImpl.hasLocalReference);
        }

        // repeated maps only count when at least one element exists
        if (map.kind === "repeated") {
            return map.count > 0 && TraceMapImpl.hasLocalReference(map.element);
        }

        return map.variants.some((variant) => TraceMapImpl.hasLocalReference(variant.map));
    },

    /** Return whether this map can reach shared heap references. */
    hasSharedReference(map: TraceMap): boolean {
        if (map.kind === "empty") {
            return false;
        }

        // fixed maps carry direct shared offsets
        if (map.kind === "fixed") {
            return map.sharedOffsets.length > 0;
        }

        // nested maps delegate to their child map
        if (map.kind === "nested") {
            return TraceMapImpl.hasSharedReference(map.map);
        }

        // composite maps aggregate child maps
        if (map.kind === "composite") {
            return map.maps.some(TraceMapImpl.hasSharedReference);
        }

        // repeated maps only count when at least one element exists
        if (map.kind === "repeated") {
            return map.count > 0 && TraceMapImpl.hasSharedReference(map.element);
        }

        return map.variants.some((variant) => TraceMapImpl.hasSharedReference(variant.map));
    },

    /** Return whether this map requires reading payload tags while scanning. */
    hasTaggedReference(map: TraceMap): boolean {
        if (map.kind === "empty" || map.kind === "fixed") {
            return false;
        }

        // nested maps delegate to their child map
        if (map.kind === "nested") {
            return TraceMapImpl.hasTaggedReference(map.map);
        }

        // composite maps aggregate child maps
        if (map.kind === "composite") {
            return map.maps.some(TraceMapImpl.hasTaggedReference);
        }

        // repeated maps delegate to their element map
        if (map.kind === "repeated") {
            return TraceMapImpl.hasTaggedReference(map.element);
        }

        return map.variants.some((variant) => TraceMapImpl.hasReference(variant.map));
    },
};

export const TraceTableImpl = {
    /** Return one trace map by id. */
    trace(table: TraceTable, id: TraceId): TraceMap | undefined {
        return table.traces[TraceIdImpl.index(id)];
    },

    /** Return all trace maps. */
    traceMaps(table: TraceTable): ReadonlyArray<TraceMap> {
        return table.traces;
    },
};

export const TraceIdImpl = {
    /** Return the raw non-zero trace identifier value. */
    raw(id: TraceId): number {
        return id;
    },

    /** Return the zero-based trace table index. */
    index(id: TraceId): number {
        return id - 1;
    },
};
