import { Intrinsic, defineObject } from "@destack/object";
import { space } from "./space.ts";

/** The sessions of changes to a space's objects keeping history, served with the objects they record. */
export const activity = defineObject(Intrinsic.activity(space));

/** The named points in the history of a space's objects, served with the objects they mark. */
export const checkpoint = defineObject(Intrinsic.checkpoint(space));
