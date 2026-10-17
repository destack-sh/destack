import { defineObject, Intrinsic } from "@destack/object";
import { RelationshipDefinition, RoleDefinition } from "../declare/permission.ts";
import { space } from "@destack/account/object";

/** A named set of permissions defined in a space. */
export const role = defineObject(
    {
        ...Intrinsic.role.definition(space),
        declarable: { schema: RoleDefinition },
    },
    Intrinsic.role.methods,
);

/** A role bound or a relation granted on an object of a space. */
export const relationship = defineObject(
    {
        ...Intrinsic.relationship.definition(space),
        declarable: { schema: RelationshipDefinition },
    },
    Intrinsic.relationship.methods,
);

/** A space's grant of some of its authority to a delegate, such as one of its installations. */
export const grant = defineObject(Intrinsic.grant.definition(space), Intrinsic.grant.methods);

/** A pending grant on an object of a space. */
export const invitation = defineObject(
    Intrinsic.invitation.definition(space),
    Intrinsic.invitation.methods,
);
