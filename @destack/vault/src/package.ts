import { definePackage } from "@destack/runtime";
import { secret, secretVersion, vault } from "./object/index.ts";

/** The vault package, which machines serve and clients call. */
export default definePackage({ objects: { vault, secret, version: secretVersion } });
