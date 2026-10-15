import type { ClientOf } from "@destack/runtime/client";
import type stackPackage from "../package.ts";

/** A client of the stack package. */
export type StackClient = ClientOf<typeof stackPackage>;
