import { schema } from "@destack/schema";

/** An S3 access key and its secret. */
export const S3Credentials = schema.object({
    /** The public access key identifier. */
    accessKeyId: schema.string().min(1),
    /** The secret signing key. */
    secretAccessKey: schema.string().min(1),
});

/** An S3 access key and its secret. */
export type S3Credentials = schema.Infer<typeof S3Credentials>;
