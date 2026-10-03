import { schema } from "@destack/schema";
import { defineObject } from "@destack/object";
import { SpaceResource } from "@destack/space/declare";
import { space } from "@destack/space/object";
import { BucketKind } from "../declare/bucket.ts";
import { Lease } from "@destack/resource";
import { FileInput, FileMetadata, FilePage } from "./file.ts";

/** A space's bucket of files: a resource of the bucket kind. */
export const bucket = defineObject({
    name: "bucket",
    plural: "buckets",
    scope: space,
    declarable: { schema: SpaceResource },
    provisioned: { kind: BucketKind },
    fields: {},
    administration: ["read"],
    methods: (method) => ({
        // read files, and presign their transfers
        files: method.query({
            permission: "read",
            input: FileInput.files,
            output: FilePage,
        }),
        file: method.query({
            permission: "read",
            input: FileInput.file,
            output: FileMetadata.nullable(),
        }),
        open: method.query({
            permission: "read",
            input: FileInput.open,
            output: Lease,
        }),
        uploadPart: method.query({
            permission: "write",
            input: FileInput.uploadPart,
            output: Lease,
        }),

        // change files, directly or in parts
        remove: method.mutation({
            permission: "write",
            input: FileInput.remove,
            output: schema.object({}),
        }),
        createUpload: method.mutation({
            permission: "write",
            input: FileInput.createUpload,
            output: schema.object({ uploadId: schema.string(), key: schema.string() }),
        }),
        completeUpload: method.mutation({
            permission: "write",
            input: FileInput.completeUpload,
            output: FileMetadata,
        }),
        abortUpload: method.mutation({
            permission: "write",
            input: FileInput.abortUpload,
            output: schema.object({}),
        }),
    }),
});
