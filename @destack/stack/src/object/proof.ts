import { defineObject, Intrinsic } from "@destack/object";
import { space } from "@destack/account/object";

/** The installation assertions a space spent, kept until they expire. */
export const proof = defineObject(Intrinsic.proof.definition(space)).methods(
    Intrinsic.proof.methods,
);
