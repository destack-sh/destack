import { defineObject, Intrinsic } from "@destack/object";
import { account } from "./account.ts";

/** A named set of permissions defined in an account. */
export const role = defineObject(Intrinsic.role(account));

/** A relation or role binding on an account or an object in it. */
export const relationship = defineObject(Intrinsic.relationship(account));

/** A proposed relationship on an account or an object in it, such as an invitation. */
export const proposal = defineObject(Intrinsic.proposal(account));
