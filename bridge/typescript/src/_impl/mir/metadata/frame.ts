import type {
    FrameLayout,
    FrameLayoutId,
    FrameMaterialization,
    FrameSlot,
    FrameSlotId,
    FrameStateId,
    FrameTable,
} from "../../../_generated/mir/metadata/frame.js";

export const FrameTableImpl = {
    /** Return one frame layout by id. */
    layout(table: FrameTable, layout: FrameLayoutId): FrameLayout | undefined {
        return table.layouts[layout];
    },

    /** Return one frame materialization by state id. */
    materialization(table: FrameTable, state: FrameStateId): FrameMaterialization | undefined {
        return table.materializations[state];
    },
};

export const FrameMaterializationImpl = {
    /** Return each source frame slot once. */
    slots(materialization: FrameMaterialization): ReadonlyArray<FrameSlotId> {
        return materialization.copiedSlots;
    },
};

export const FrameLayoutIdImpl = {
    /** Return the raw frame layout id. */
    raw(id: FrameLayoutId): number {
        return id;
    },
};

export const FrameSlotIdImpl = {
    /** Return the raw frame slot id. */
    raw(id: FrameSlotId): number {
        return id;
    },
};

export const FrameStateIdImpl = {
    /** Return the raw frame state id. */
    raw(id: FrameStateId): number {
        return id;
    },
};

export const FrameLayoutImpl = {
    /** Return one frame slot by id. */
    slot(layout: FrameLayout, id: FrameSlotId): FrameSlot | undefined {
        return layout.slots[id];
    },

    /** Return the frame slot id for one SSA value. */
    valueSlotId(layout: FrameLayout, value: number): FrameSlotId | undefined {
        return value < layout.valueCount ? value : undefined;
    },

    /** Return the frame slot id for one local. */
    localSlotId(layout: FrameLayout, local: number): FrameSlotId | undefined {
        return local < layout.localCount ? layout.valueCount + local : undefined;
    },

    /** Return the value slot at one SSA value index. */
    value(layout: FrameLayout, value: number): FrameSlot | undefined {
        return value < layout.valueCount ? layout.slots[value] : undefined;
    },

    /** Return the local slot at one local index. */
    local(layout: FrameLayout, local: number): FrameSlot | undefined {
        if (local >= layout.localCount) {
            return undefined;
        }

        const index = layout.valueCount + local;

        return layout.slots[index];
    },

    /** Return the SSA value addressed by one slot id. */
    valueForSlot(layout: FrameLayout, id: FrameSlotId): number | undefined {
        return id < layout.valueCount ? id : undefined;
    },

    /** Return the local addressed by one slot id. */
    localForSlot(layout: FrameLayout, id: FrameSlotId): number | undefined {
        if (id < layout.valueCount) {
            return undefined;
        }

        const local = id - layout.valueCount;

        return local < layout.localCount ? local : undefined;
    },

    /** Return whether one slot id addresses the callable environment. */
    isEnvironmentSlot(layout: FrameLayout, id: FrameSlotId): boolean {
        return layout.environmentSlot === id;
    },

    /** Return the callable environment slot when present. */
    environment(layout: FrameLayout): FrameSlot | undefined {
        return layout.environmentSlot === undefined
            ? undefined
            : FrameLayoutImpl.slot(layout, layout.environmentSlot);
    },

    /** Return all value slots. */
    values(layout: FrameLayout): ReadonlyArray<FrameSlot> {
        return layout.slots.slice(0, layout.valueCount);
    },

    /** Return all local slots. */
    locals(layout: FrameLayout): ReadonlyArray<FrameSlot> {
        const start = layout.valueCount;
        const end = start + layout.localCount;

        return layout.slots.slice(start, end);
    },

    /** Return the frame slot count. */
    slotLen(layout: FrameLayout): number {
        return layout.slots.length;
    },

    /** Return all frame slot ids. */
    slotIds(layout: FrameLayout): ReadonlyArray<FrameSlotId> {
        return layout.slots.map((_slot, value) => value);
    },
};
