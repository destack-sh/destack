import { defineObject, Intrinsic } from "@destack/object";
import { Scope } from "@destack/sync";

/** The key proofs spent at the universe, kept until they expire. */
export const proof = defineObject(Intrinsic.proof.definition(Scope.universe.id)).methods(
    Intrinsic.proof.methods,
);
