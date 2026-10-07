import type * as Cloudflare from "@cloudflare/workers-types";
import type {
    BucketBody,
    BucketFile,
    MultipartUpload,
    UploadedPart,
    UploadPartOptions,
} from "../bucket/index.ts";
import type { R2OperationCount } from "./bucket.ts";
import { R2Body } from "./body.ts";
import { R2File } from "./file.ts";

/** A multipart upload supplied by an R2 binding. */
export class R2MultipartUpload implements MultipartUpload {
    /** The authorised R2 upload. */
    readonly #upload: Cloudflare.R2MultipartUpload;
    /** The operations of the bucket sending the upload, which its parts and completion add to. */
    readonly #operations: R2OperationCount;

    /** Use an R2 upload handle, counting its operations with its bucket's. */
    constructor(upload: Cloudflare.R2MultipartUpload, operations: R2OperationCount) {
        this.#upload = upload;
        this.#operations = operations;
    }

    /** The destination key. */
    get key(): string {
        return this.#upload.key;
    }
    /** The upload identifier. */
    get uploadId(): string {
        return this.#upload.uploadId;
    }

    /** Store or replace a numbered part. */
    uploadPart(
        partNumber: number,
        body: BucketBody,
        options?: UploadPartOptions,
    ): Promise<UploadedPart> {
        this.#operations.classA += 1;

        return this.#upload.uploadPart(partNumber, R2Body.write(body), options);
    }

    /** Assemble the selected part uploads into one file. */
    async complete(uploaded: UploadedPart[]): Promise<BucketFile> {
        this.#operations.classA += 1;

        return R2File.describe(await this.#upload.complete(uploaded));
    }

    /** Discard the upload. */
    abort(): Promise<void> {
        return this.#upload.abort();
    }
}
