import { expect, test } from "@destack/test";
import { createDropzone, createFilePicker, createFileUploader } from "./upload.ts";

test("choose, receive and upload nothing on the server", async () => {
    const picker = createFilePicker();
    const zone = createDropzone();
    const uploader = createFileUploader(async () => {});

    expect([
        picker.files(),
        zone.isDragging(),
        uploader.status(),
        await uploader.upload([]),
    ]).toEqual([[], false, "idle", []]);
});
