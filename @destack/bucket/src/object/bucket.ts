import { relation } from "@destack/access";
import { schema } from "@destack/schema";
import { defineObject, method } from "@destack/object";
import { SpaceResource } from "@destack/space/declare";
import { Resource, space } from "@destack/space/object";
import { BucketKind } from "../declare/bucket.ts";
import { Lease } from "@destack/resource";
import { FileInput, FileMetadata, FilePage } from "./file.ts";

/** A space's bucket of files: a resource of the bucket kind. */
export const bucket = defineObject({
    name: "bucket",
    plural: "buckets",
    scope: space,
    controlled: true,
    declarable: { schema: SpaceResource },
    fields: Resource.fields(BucketKind),
    indexes: Resource.indexes,
    constraints: Resource.constraints("bucket"),
    relations: Resource.relations,
    permissions: {
        ...Resource.permissions,

        files: relation("user"),
        download: relation("user"),
        upload: relation("user"),
        remove: relation("user"),
    },
    administration: ["read"],
    methods: {
        ...Resource.methods,

        // read files, and presign their transfers
        files: method({
            permission: "files",
            mutates: false,
            input: FileInput.files,
            output: FilePage,
        }),
        file: method({
            permission: "files",
            mutates: false,
            input: FileInput.file,
            output: FileMetadata.nullable(),
        }),
        open: method({
            permission: "download",
            mutates: false,
            input: FileInput.open,
            output: Lease,
        }),
        uploadPart: method({
            permission: "upload",
            mutates: false,
            input: FileInput.uploadPart,
            output: Lease,
        }),

        // change files, directly or in parts
        remove: method({
            permission: "remove",
            input: FileInput.remove,
            output: schema.object({}),
        }),
        createUpload: method({
            permission: "upload",
            input: FileInput.createUpload,
            output: schema.object({ uploadId: schema.string(), key: schema.string() }),
        }),
        completeUpload: method({
            permission: "upload",
            input: FileInput.completeUpload,
            output: FileMetadata,
        }),
        abortUpload: method({
            permission: "upload",
            input: FileInput.abortUpload,
            output: schema.object({}),
        }),
    },
});
