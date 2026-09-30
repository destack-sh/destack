import { defineObject, Intrinsic } from "@destack/object";
import { SpaceRelationship, SpaceRole } from "../declare/permission.ts";
import { space } from "./space.ts";

/** A named set of permissions defined in a space. */
export const role = defineObject({ ...Intrinsic.role(space), declarable: { schema: SpaceRole } });

/** A role bound or a relation granted on an object of a space. */
export const relationship = defineObject({
    ...Intrinsic.relationship(space),
    declarable: { schema: SpaceRelationship },
});
