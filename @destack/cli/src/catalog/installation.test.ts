import { expect, refusal, test } from "@destack/test";
import { PackageSpecifier } from "./installation.ts";

test("read a package's release or tag and a repository's branch, refusing one without a branch", async () => {
    expect({
        latest: PackageSpecifier.read("@ada/notes"),
        pinned: PackageSpecifier.read("@ada/notes@2026.11.0"),
        root: PackageSpecifier.read("@ada/notes#main"),
        below: PackageSpecifier.read("@ada/apps/tasks/app#feature/board"),
        bare: await refusal(() => PackageSpecifier.read("@ada/notes#")),
    }).toEqual({
        latest: { kind: "release", name: "@ada/notes", release: "latest" },
        pinned: { kind: "release", name: "@ada/notes", release: "2026.11.0" },
        root: { kind: "branch", repository: "@ada/notes", directory: ".", branch: "main" },
        below: {
            kind: "branch",
            repository: "@ada/apps",
            directory: "tasks/app",
            branch: "feature/board",
        },
        bare: [
            "BAD_REQUEST",
            "USAGE_INVALID",
            "repository '@ada/notes#' is invalid: use @handle/repository[/directory]#branch",
        ],
    });
});
