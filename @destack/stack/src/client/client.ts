import type { ClientOf } from "@destack/object/client";
import type stackPackage from "../package.ts";

/** A client of the stack package. */
export type StackClient = ClientOf<typeof stackPackage>;
