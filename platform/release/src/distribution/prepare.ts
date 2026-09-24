import { appendFile } from "node:fs/promises";
import { Release } from "@destack/update/release";
import { run } from "./command.ts";

/** Select one version before building or exposing signing credentials. */
async function prepare(): Promise<void> {
    // distinguish stable tags from automatic nightly publication
    const isTag = process.env.GITHUB_REF_TYPE === "tag";
    const shouldPublish =
        isTag ||
        process.env.GITHUB_EVENT_NAME === "schedule" ||
        process.env.DESTACK_PUBLISH === "true";
    const channel = isTag ? "stable" : "nightly";
    const now = new Date();
    const sequence = process.env.GITHUB_RUN_NUMBER;
    const attempt = process.env.GITHUB_RUN_ATTEMPT;
    if (!sequence || !attempt || !/^[1-9]\d*$/.test(sequence) || !/^[1-9]\d*$/.test(attempt)) {
        throw new Error("release preparation requires a GitHub run number and attempt");
    }

    // retain the stable tag or derive a unique calendar nightly version
    const tag = process.env.GITHUB_REF_NAME;
    const version = isTag
        ? tag?.replace(/^v/, "")
        : `${now.getUTCFullYear()}.${now.getUTCMonth() + 1}.${sequence}-nightly.${attempt}`;
    const release = new Release(version, "aarch64-apple-darwin");
    if (release.channel !== channel || (isTag && tag !== `v${release.version}`)) {
        throw new Error("release tag must select a stable calendar version");
    }

    // require published source to belong to the default branch
    if (shouldPublish) {
        if (!isTag && process.env.GITHUB_REF !== "refs/heads/main") {
            throw new Error("nightly publication requires main");
        }
        await run("git", ["merge-base", "--is-ancestor", "HEAD", "origin/main"]);
    }

    // expose identical public build choices to every target
    const output = process.env.GITHUB_OUTPUT;
    if (!output) {
        throw new Error("release preparation requires GITHUB_OUTPUT");
    }
    await appendFile(
        output,
        `version=${release.version}\npublish=${shouldPublish}\nchannel=${channel}\n`,
    );
    console.log(
        `Destack ${release.version}: ${shouldPublish ? "signed publication" : "build verification"}`,
    );
}

await prepare();
