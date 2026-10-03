import type * as Cloudflare from "@cloudflare/workers-types";
import type {
    BucketBody,
    BucketFile,
    MultipartUpload,
    UploadedPart,
    UploadPartOptions,
} from "../bucket/index.ts";
import { R2Body } from "./body.ts";
import { R2File } from "./file.ts";

/** A multipart upload supplied by an R2 binding. */
export class R2MultipartUpload implements MultipartUpload {
    /** The authorised R2 upload. */
    readonly #upload: Cloudflare.R2MultipartUpload;

    /** Use an R2 upload handle. */
    constructor(upload: Cloudflare.R2MultipartUpload) {
        this.#upload = upload;
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
        return this.#upload.uploadPart(partNumber, R2Body.write(body), options);
    }

    /** Assemble the selected part uploads into one file. */
    async complete(uploaded: UploadedPart[]): Promise<BucketFile> {
        return R2File.describe(await this.#upload.complete(uploaded));
    }

    /** Discard the upload. */
    abort(): Promise<void> {
        return this.#upload.abort();
    }
}
