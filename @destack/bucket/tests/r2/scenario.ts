import type * as Cloudflare from "@cloudflare/workers-types";
import { R2Bucket } from "../../src/r2/index.ts";
import {
    exerciseContents,
    exerciseConditions,
    exerciseListing,
    exerciseGroups,
    exerciseMarkers,
} from "../scenario/bucket.ts";
import { exerciseMultipart } from "../scenario/multipart.ts";

/** The storage class R2 reports for files stored without one. */
const DEFAULT_STORAGE_CLASS = "Standard";

/** The Worker running the bucket scenarios against the local R2 simulator. */
export default {
    /** Run each scenario against a simulated R2 binding. */
    async fetch(request: Request, environment: { BUCKET: Cloudflare.R2Bucket }): Promise<Response> {
        const bucket = new R2Bucket(fillStorageClass(environment.BUCKET));
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

/** Fill in the storage class the simulator leaves empty on what R2 calls return; the hosted test:r2 run reads R2's own. */
function fillStorageClass<Value>(value: Value): Value;
/**
 * Fill in the storage class on one value.
 *
 * @construct the proxy answers every property as the value does, with the default class in place of an empty one, so the value keeps its type.
 */
function fillStorageClass(value: unknown): unknown {
    // pass values through, and fill results once they resolve
    if (typeof value !== "object" || value === null) {
        return value;
    } else if (value instanceof Promise) {
        return value.then((resolved: unknown) => fillStorageClass(resolved));
    }

    return new Proxy(value, {
        get(target, property) {
            const field: unknown = Reflect.get(target, property, target);
            // report the default class in place of the empty one
            if (property === "storageClass") {
                return field === "" ? DEFAULT_STORAGE_CLASS : field;
            }
            // fill each listed file
            else if (property === "objects") {
                if (!Array.isArray(field)) {
                    throw new TypeError("listed objects are no array");
                }

                return field.map((item: unknown) => fillStorageClass(item));
            }
            // fill what each call returns
            else if (typeof field === "function") {
                return (...parameters: unknown[]) => {
                    const result: unknown = field.apply(target, parameters);

                    return fillStorageClass(result);
                };
            }
            // pass other fields through
            else {
                return field;
            }
        },
    });
}
