import type { Deriver } from "@destack/identity";
import { schema } from "@destack/schema";

/** The label of a space's S3 secret, distinct from every other key its root secret derives. */
const LABEL = "destack s3 v1";

/** An S3 access key and its secret. */
const s3Credentials = schema.object({
    /** The public access key identifier. */
    accessKeyId: schema.string().min(1),
    /** The secret signing key. */
    secretAccessKey: schema.string().min(1),
});

/** An S3 access key and its secret. */
export const S3Credentials = Object.assign(s3Credentials, {
    /** Derive a space's credentials from its root secret: the space as the access key, a derived secret as the secret key. */
    async derive(root: Pick<Deriver, "derive">, space: string): Promise<S3Credentials> {
        return { accessKeyId: space, secretAccessKey: (await root.derive(LABEL)).toHex() };
    },
});

/** An S3 access key and its secret. */
export type S3Credentials = schema.Infer<typeof s3Credentials>;
