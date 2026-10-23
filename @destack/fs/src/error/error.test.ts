import { expect, test } from "@destack/test";
import { FileSystemError } from "./error.ts";

test("report every host filesystem failure as an internal call error", () => {
    const failures = [
        new FileSystemError("read", "/notes/today.md", "ENOENT"),
        new FileSystemError("write", "/notes/today.md", "EACCES"),
    ];
    expect(failures.map((failure) => failure.toCallError())).toEqual([
        {
            code: "INTERNAL_SERVER_ERROR",
            reason: "FILE_SYSTEM_FAILED",
            domain: "@destack/fs",
            message: "cannot read /notes/today.md: ENOENT",
        },
        {
            code: "INTERNAL_SERVER_ERROR",
            reason: "FILE_SYSTEM_FAILED",
            domain: "@destack/fs",
            message: "cannot write /notes/today.md: EACCES",
        },
    ]);
});
