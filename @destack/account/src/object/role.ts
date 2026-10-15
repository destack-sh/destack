import { defineObject, Intrinsic } from "@destack/object";
import { account } from "./account.ts";

/** A named set of permissions defined in an account. */
export const role = defineObject(Intrinsic.role.definition(account)).methods(
    Intrinsic.role.methods,
);
