import { expect, test } from "@destack/test";
import { FileSystemError } from "./error.ts";

test("report every host filesystem failure as an internal service error", () => {
    const failures = [
        new FileSystemError("read", "/notes/today.md", "ENOENT"),
        new FileSystemError("write", "/notes/today.md", "EACCES"),
    ];
    expect(failures.map((failure) => failure.toServiceError())).toEqual([
        { code: "INTERNAL_SERVER_ERROR", message: "cannot read /notes/today.md: ENOENT" },
        { code: "INTERNAL_SERVER_ERROR", message: "cannot write /notes/today.md: EACCES" },
    ]);
});
