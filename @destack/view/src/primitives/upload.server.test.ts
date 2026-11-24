import { expect, test } from "@destack/test";
import { createDropzone, createFilePicker } from "./upload.ts";

test("choose and receive nothing on the server", () => {
    const picker = createFilePicker();
    const zone = createDropzone();

    expect([picker.files(), zone.isDragging()]).toEqual([[], false]);
});
