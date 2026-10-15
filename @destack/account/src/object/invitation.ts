import { defineObject, Intrinsic } from "@destack/object";
import { Scope } from "@destack/sync";
import { account } from "./account.ts";
import { organisation } from "./organisation.ts";

/** A pending grant on an object of an account, an organisation or the universe. */
export const invitation = defineObject(
    Intrinsic.invitation.definition([account, organisation, Scope.universe.id]),
).methods(Intrinsic.invitation.methods);
