import { join } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { read, run } from "../distribution/command.ts";
import { CHECKOUT, readCommit } from "../distribution/distribution.ts";

/** Record a published release as its version tag and a GitHub Release listing its commits. */
export async function recordRelease(environment: Environment<HostCapability>): Promise<void> {
    // read the published version and the commit it was built from
    const version = environment.process.variables["DESTACK_RELEASE_VERSION"];
    if (version === undefined || version === "") {
        throw new Error("recording a release requires DESTACK_RELEASE_VERSION");
    }
    const commit = await readCommit(CHECKOUT, environment);

    // list the commits since the previous release's tag, or call it the first
    const tags = await read(
        "git",
        ["tag", "--list", "v*", "--merged", commit, "--sort=-v:refname"],
        environment,
        CHECKOUT,
    );
    const previous = tags.split("\n").find((tag) => tag !== "");
    const log = ["log", "--format=- %s", `${previous}..${commit}`];
    const notes =
        previous === undefined
            ? "The first release.\n"
            : await read("git", log, environment, CHECKOUT);

    // tag the commit and publish the release with its notes
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("destack-record-");
    try {
        const file = join(directory, "notes.md");
        await files.writeTextFile(file, notes);
        await run(
            "gh",
            [
                "release",
                "create",
                `v${version}`,
                "--target",
                commit,
                "--title",
                `Destack ${version}`,
                "--notes-file",
                file,
            ],
            environment,
        );
    } finally {
        await files.remove(directory, { recursive: true, force: true });
    }
}
