import { present } from "@destack/schema";
import {
    AbortMultipartUploadCommand,
    CompleteMultipartUploadCommand,
    CreateMultipartUploadCommand,
    GetObjectCommand,
    ListMultipartUploadsCommand,
    ListPartsCommand,
    PutObjectCommand,
    UploadPartCommand,
    UploadPartCopyCommand,
} from "@aws-sdk/client-s3";
import { expect, test } from "@destack/test";
import { failure, S3Fixture } from "./test/fixture.ts";

/** The smallest size of every multipart part but the last, five MiB. */
const PART_SIZE = 5 * 1024 * 1024;

test("assemble, list and abort multipart uploads through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client({ endpoint: "https://s3.test" });
    await client.send(
        new PutObjectCommand({ Bucket: "files", Key: "tail.txt", Body: "0123456789" }),
    );

    // create an upload with the metadata its file receives, and one to abort
    const { UploadId: uploadId } = await client.send(
        new CreateMultipartUploadCommand({
            Bucket: "files",
            Key: "video.bin",
            ContentType: "video/mp4",
        }),
    );
    const { UploadId: abortedId } = await client.send(
        new CreateMultipartUploadCommand({ Bucket: "files", Key: "draft.bin" }),
    );

    // stream a full part unsigned, and copy the final part from a range of another file
    const first = new Uint8Array(PART_SIZE).fill(97);
    const part = await client.send(
        new UploadPartCommand({
            Bucket: "files",
            Key: "video.bin",
            UploadId: uploadId,
            PartNumber: 1,
            Body: new Blob([first]).stream(),
            ContentLength: PART_SIZE,
        }),
    );
    const copied = await client.send(
        new UploadPartCopyCommand({
            Bucket: "files",
            Key: "video.bin",
            UploadId: uploadId,
            PartNumber: 2,
            CopySource: "files/tail.txt",
            CopySourceRange: "bytes=2-5",
        }),
    );

    // list the parts and both uploads
    const parts = await client.send(
        new ListPartsCommand({ Bucket: "files", Key: "video.bin", UploadId: uploadId }),
    );
    expect(parts.Parts?.map((entry) => [entry.PartNumber, entry.ETag, entry.Size])).toEqual([
        [1, part.ETag, PART_SIZE],
        [2, present(copied.CopyPartResult, "CopyPartResult").ETag, 4],
    ]);
    const uploads = await client.send(new ListMultipartUploadsCommand({ Bucket: "files" }));
    expect(
        uploads.Uploads?.map((entry) => [entry.Key, entry.UploadId, entry.StorageClass]),
    ).toEqual([
        ["draft.bin", abortedId, "STANDARD"],
        ["video.bin", uploadId, "STANDARD"],
    ]);

    // refuse parts out of order, then complete the upload
    const complete = (partNumbers: number[]) =>
        client.send(
            new CompleteMultipartUploadCommand({
                Bucket: "files",
                Key: "video.bin",
                UploadId: uploadId,
                MultipartUpload: {
                    Parts: partNumbers.map((number) => ({
                        PartNumber: number,
                        ETag:
                            number === 1
                                ? part.ETag
                                : present(copied.CopyPartResult, "CopyPartResult").ETag,
                    })),
                },
            }),
        );
    expect(await failure(complete([2, 1]))).toEqual({ name: "InvalidPartOrder", status: 400 });
    const completed = await complete([1, 2]);
    const file = present(
        await present(fixture.buckets.get("files"), "files").head("video.bin"),
        "file",
    );
    expect({
        etag: completed.ETag,
        size: file.size,
        type: file.httpMetadata.contentType,
    }).toEqual({
        etag: file.httpEtag,
        size: PART_SIZE + 4,
        type: "video/mp4",
    });
    const tail = await client.send(
        new GetObjectCommand({ Bucket: "files", Key: "video.bin", Range: "bytes=-5" }),
    );
    expect(await present(tail.Body, "Body").transformToString()).toBe("a2345");

    // abort the other upload, after which its parts are gone
    await client.send(
        new AbortMultipartUploadCommand({ Bucket: "files", Key: "draft.bin", UploadId: abortedId }),
    );
    expect(
        await failure(
            client.send(
                new ListPartsCommand({ Bucket: "files", Key: "draft.bin", UploadId: abortedId }),
            ),
        ),
    ).toEqual({
        name: "NoSuchUpload",
        status: 404,
    });
});
