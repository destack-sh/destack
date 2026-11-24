import { expect, test } from "@destack/test";
import { MediaType } from "./index.ts";

test("read a file's media type by its extension, in either case", () => {
    const types = ["a.html", "photos/Tram.JPG", "a.tar.gz", "a.unknown", "Makefile"].map((path) =>
        MediaType.of(path),
    );

    expect(types).toEqual([
        "text/html",
        "image/jpeg",
        "application/gzip",
        "application/octet-stream",
        "application/octet-stream",
    ]);
});
