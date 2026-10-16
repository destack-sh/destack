import { defineObject, Intrinsic } from "@destack/object";
import { Scope } from "@destack/sync";
import { account } from "./account.ts";
import { organisation } from "./organisation.ts";

/** A relation or role binding on an object of an account, an organisation or the universe. */
export const relationship = defineObject(
    Intrinsic.relationship.definition([account, organisation, Scope.universe.id]),
    Intrinsic.relationship.methods,
);
