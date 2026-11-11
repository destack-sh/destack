import type { Select } from "@destack/db";
import { schema } from "@destack/schema";
import { defineObject } from "@destack/object";
import { BucketKind } from "@destack/bucket";
import { space } from "@destack/account/object";
import { Lease } from "@destack/resource";
import { FileInput, FileMetadata, FilePage } from "./file.ts";
import { provisioned } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A space's bucket of files: a resource of the bucket kind. */
export const bucket = defineObject(
    {
        name: "bucket",
        plural: "buckets",
        title: t`${plural(argument("count"), { one: "Bucket", other: "Buckets" })}`,
        scope: space,
        fields: {},
        access: {
            suspended: ["read"],
        },
        traits: [provisioned({ kind: BucketKind })],
    },
    (method) => ({
        // read files, and presign their transfers
        files: method.query({
            title: t`List files`,
            permission: "read",
            input: FileInput.files,
            output: FilePage,
        }),
        file: method.query({
            title: t`Read file`,
            permission: "read",
            input: FileInput.file,
            output: FileMetadata.nullable(),
        }),
        open: method.query({
            title: t`Open`,
            permission: "read",
            input: FileInput.open,
            output: Lease,
        }),
        uploadPart: method.query({
            title: t`Upload part`,
            permission: "write",
            input: FileInput.uploadPart,
            output: Lease,
        }),

        // change files, directly or in parts
        remove: method.mutation({
            title: t`Remove`,
            permission: "write",
            input: FileInput.remove,
            output: schema.object({}),
        }),
        createUpload: method.mutation({
            title: t`Create upload`,
            permission: "write",
            input: FileInput.createUpload,
            output: schema.object({ uploadId: schema.string(), key: schema.string() }),
        }),
        completeUpload: method.mutation({
            title: t`Complete upload`,
            permission: "write",
            input: FileInput.completeUpload,
            output: FileMetadata,
        }),
        abortUpload: method.mutation({
            title: t`Abort upload`,
            permission: "write",
            input: FileInput.abortUpload,
            output: schema.object({}),
        }),
    }),
);

/** A space's bucket of files. */
export type Bucket = Select<typeof bucket.table>;
