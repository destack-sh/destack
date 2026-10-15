import { definePackage } from "@destack/runtime";
import { call } from "./audit/call.ts";

/** The handle a space and a machine serve their call log through. */
export default definePackage({ objects: { call } });
