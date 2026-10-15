import { defineObject, Intrinsic } from "@destack/object";
import { user } from "./user.ts";

/** A person's grant of permissions to a principal acting for them, kept in their own scope. */
export const grant = defineObject(Intrinsic.grant.definition(user)).methods(
    Intrinsic.grant.methods,
);
