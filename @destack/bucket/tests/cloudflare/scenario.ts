import type * as Cloudflare from "@cloudflare/workers-types";
import { R2Bucket } from "../../src/cloudflare/bucket.ts";
import {
    exerciseContents,
    exerciseConditions,
    exerciseListing,
    exerciseGroups,
    exerciseMarkers,
} from "../scenario/bucket.ts";
import { exerciseMultipart } from "../scenario/multipart.ts";
import { exerciseBlobStore } from "../scenario/store.ts";
import { exerciseOperations } from "./operations.ts";

/** The Worker running the bucket scenarios against the local R2 simulator. */
export default {
    /** Run each scenario against a simulated R2 binding. */
    async fetch(request: Request, environment: { BUCKET: Cloudflare.R2Bucket }): Promise<Response> {
        const bucket = new R2Bucket(environment.BUCKET);
        try {
            switch (new URL(request.url).pathname) {
                case "/exerciseContents":
                    await exerciseContents(bucket);
                    break;
                case "/exerciseConditions":
                    await exerciseConditions(bucket, true);
                    break;
                case "/exerciseListing":
                    await exerciseListing(bucket);
                    break;
                case "/exerciseGroups":
                    await exerciseGroups(bucket);
                    break;
                case "/exerciseMarkers":
                    await exerciseMarkers(bucket);
                    break;
                case "/exerciseMultipart":
                    await exerciseMultipart(bucket);
                    break;
                case "/exerciseBlobStore":
                    await exerciseBlobStore(bucket);
                    break;
                case "/exerciseOperations":
                    await exerciseOperations(bucket);
                    break;
                default:
                    throw new Error("unknown bucket scenario");
            }
            return new Response("ok");
        } catch (error) {
            return new Response(error instanceof Error ? error.stack : String(error), {
                status: 500,
            });
        }
    },
};
