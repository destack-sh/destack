import type { ReferenceMeta } from "../../../_generated/program/vm/meta.js";
import type { Access, Nullability, ReferenceKind, Space } from "../../../_generated/mir/tree/type.js";

/** Bit mask for the reference kind field. */
const KIND_MASK = 0x7;
/** Bit offset for the access field. */
const ACCESS_SHIFT = 3;
/** Bit mask for the access field. */
const ACCESS_MASK = 0x3 << ACCESS_SHIFT;
/** Bit offset for the nullability field. */
const NULLABILITY_SHIFT = 5;
/** Bit mask for the nullability field. */
const NULLABILITY_MASK = 0x3 << NULLABILITY_SHIFT;
/** Bit offset for the space field. */
const SPACE_SHIFT = 7;
/** Bit mask for the space field. */
const SPACE_MASK = 0x7 << SPACE_SHIFT;

export const ReferenceMetaImpl = {
    /** Return the encoded reference kind. */
    kind(meta: ReferenceMeta): ReferenceKind | undefined {
        const bits = meta.bits & KIND_MASK;

        // managed references use kind tag 1
        if (bits === 1) {
            return "managed";
        }
        // unique references use kind tag 2
        else if (bits === 2) {
            return "unique";
        }
        // borrowed references use kind tag 3
        else if (bits === 3) {
            return "borrowed";
        }
        // raw references use kind tag 4
        else if (bits === 4) {
            return "raw";
        }
        // invalid tags do not decode to reference kinds
        else {
            return undefined;
        }
    },

    /** Return the encoded reference access. */
    access(meta: ReferenceMeta): Access | undefined {
        // invalid reference kinds do not have access bits
        if (ReferenceMetaImpl.kind(meta) === undefined) {
            return undefined;
        }

        const bits = (meta.bits & ACCESS_MASK) >> ACCESS_SHIFT;

        // readonly access is encoded as 0
        if (bits === 0) {
            return "readonly";
        }
        // mutable access is encoded as 1
        else if (bits === 1) {
            return "mutable";
        }
        // exclusive access is encoded as 2
        else if (bits === 2) {
            return "exclusive";
        }
        // invalid tags do not decode to access modes
        else {
            return undefined;
        }
    },

    /** Return the encoded nullability. */
    nullability(meta: ReferenceMeta): Nullability {
        const bits = (meta.bits & NULLABILITY_MASK) >> NULLABILITY_SHIFT;

        // nullability supports null only
        if (bits === 1) {
            return "null";
        }
        // nullability supports undefined only
        else if (bits === 2) {
            return "undefined";
        }
        // nullability supports both null and undefined
        else if (bits === 3) {
            return "nullOrUndefined";
        }
        // zero and invalid tags default to non-nullable
        else {
            return "none";
        }
    },

    /** Return the encoded memory space. */
    space(meta: ReferenceMeta): Space {
        const bits = (meta.bits & SPACE_MASK) >> SPACE_SHIFT;

        // frame space is encoded as 1
        if (bits === 1) {
            return "frame";
        }
        // static space is encoded as 2
        else if (bits === 2) {
            return "static";
        }
        // shared space is encoded as 3
        else if (bits === 3) {
            return "shared";
        }
        // zero and invalid tags default to local space
        else {
            return "local";
        }
    },
};
