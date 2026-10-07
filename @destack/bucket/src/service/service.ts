import { defineService } from "@destack/service";
import { bucket } from "../object/index.ts";

/** The buckets of the spaces a cell serves, served as objects with their files. */
export const bucketService = defineService("bucket", { objects: { bucket } });
