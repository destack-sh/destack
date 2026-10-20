import objectPackage from "../package.ts";
import { defineServer } from "./server.ts";

/** Serve object's package: the call log a store's journal appends to, and the access objects. */
export default defineServer(objectPackage, {});
