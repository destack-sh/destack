import { definePackage } from "@destack/runtime";
import { endpoint, message } from "./object/index.ts";

/** The handle stores serve messages and webhook endpoints through, and clients call them by. */
export default definePackage({ objects: { message, endpoint } });
