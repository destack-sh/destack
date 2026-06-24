import type {
    ElementLayout,
    Layout,
    LayoutField,
    LayoutId,
    LayoutMetadata,
    LayoutShape,
    LayoutTable,
} from "../../../_generated/mir/metadata/layout.js";
import type { LocalNodeId } from "../../../_generated/mir/tree/node.js";
import { getLocalNodeValue } from "../node.js";

export const LayoutMetadataImpl = {
    /** Return the layout entry for a type id when available. */
    typeLayout(metadata: LayoutMetadata, ty: LocalNodeId): Layout | undefined {
        const layout = LayoutMetadataImpl.layoutId(metadata, ty);

        return layout === undefined ? undefined : LayoutTableImpl.layout(metadata.layoutTable, layout);
    },

    /** Return the layout id for a type when present. */
    layoutId(metadata: LayoutMetadata, ty: LocalNodeId): LayoutId | undefined {
        return getLocalNodeValue(metadata.layoutByType, ty);
    },
};

export const LayoutTableImpl = {
    /** Return a layout entry for an id. */
    layout(table: LayoutTable, id: LayoutId): Layout | undefined {
        return table.layouts[LayoutIdImpl.index(id)];
    },
};

export const LayoutImpl = {
    /** Return the byte offset of a dynamic dispatch pointer. */
    dynamicDispatchOffset(layout: Layout): number | undefined {
        return layout.shape.kind === "dynamic" ? layout.alignment : undefined;
    },

    /** Return the byte width of this layout. */
    byteLen(layout: Layout): number {
        return layout.size;
    },

    /** Return one field by layout index. */
    fieldAt(layout: Layout, index: number): LayoutField | undefined {
        return LayoutShapeImpl.fields(layout.shape)[index];
    },

    /** Return the field count for field-addressable layouts. */
    fieldCount(layout: Layout): number | undefined {
        if (layout.shape.kind === "struct") {
            return layout.shape.struct.fields.length;
        }

        // tuple layouts address elements as fields
        if (layout.shape.kind === "tuple") {
            return layout.shape.tuple.elements.length;
        }

        // object layouts address instance fields
        if (layout.shape.kind === "object") {
            return layout.shape.object.fields.length;
        }

        return undefined;
    },
};

export const LayoutShapeImpl = {
    /** Return element layout when this shape stores indexed elements inline. */
    elements(shape: LayoutShape): ElementLayout | undefined {
        if (shape.kind === "array") {
            return shape.array;
        }

        // vectors share the same inline element layout form
        if (shape.kind === "vector") {
            return shape.vector;
        }

        return undefined;
    },

    /** Return field layouts for field-addressable shapes. */
    fields(shape: LayoutShape): ReadonlyArray<LayoutField> {
        if (shape.kind === "struct") {
            return shape.struct.fields;
        }

        // tuple layouts address elements as fields
        if (shape.kind === "tuple") {
            return shape.tuple.elements;
        }

        // object layouts address instance fields
        if (shape.kind === "object") {
            return shape.object.fields;
        }

        return [];
    },
};

export const LayoutIdImpl = {
    /** Return the raw layout identifier value. */
    raw(id: LayoutId): number {
        return id;
    },

    /** Return the zero-based layout table index. */
    index(id: LayoutId): number {
        return id - 1;
    },
};
