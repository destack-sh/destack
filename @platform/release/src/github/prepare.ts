import { appendFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { CHANNELS, type Channel, Release } from "@destack/update/release";
import { run } from "../distribution/command.ts";
import { readCommit } from "../distribution/distribution.ts";
import { listInstallers, PLATFORMS } from "../distribution/platform.ts";
import { RepositoryConfiguration } from "../repository/configuration.ts";
import { RepositoryRenewal } from "../repository/renewal.ts";
import { print } from "../output/index.ts";

/** Repository containing the release checkout. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

/** Select the version and the work of a release run before building or exposing signing credentials. */
async function prepare(): Promise<void> {
    // require the trigger's channel: stable from a version tag, nightly from a branch
    const configuration = new RepositoryConfiguration();
    const channel = configuration.channel;
    const isTag = process.env["GITHUB_REF_TYPE"] === "tag";
    if ((channel === "stable") !== isTag) {
        throw new Error("stable releases publish from a version tag and nightlies from a branch");
    }
    const release = selectRelease(channel, isTag);

    // publish only from main or a version tag on main
    const isPublishing = isTag || process.env["GITHUB_REF"] === "refs/heads/main";
    if (isPublishing) {
        await run("git", ["merge-base", "--is-ancestor", "HEAD", "origin/main"], ROOT);
    }
    const published = isPublishing ? await readPublished() : new Map<Channel, RepositoryRenewal>();

    // build a nightly only when main moved past the commit of the published nightly
    const commit = await readCommit(ROOT);
    const current = published.get(channel);
    const isBuilding =
        channel === "stable" || current === undefined || !current.isBuiltFrom(commit);
    const isPublishingBuild = isPublishing && isBuilding;

    // renew each published channel this run builds nothing for
    const renewals = [...published.keys()].filter((name) => !isBuilding || name !== channel);
    // check the expiry of every channel this run writes
    const checks = CHANNELS.filter(
        (name) => renewals.includes(name) || (isPublishingBuild && name === channel),
    );

    // expose identical public build choices to every job
    const outputs = {
        version: release.version,
        channel,
        build: String(isBuilding),
        publish: String(isPublishingBuild),
        targets: JSON.stringify(PLATFORMS.map(({ target, runner }) => ({ target, runner }))),
        signed: JSON.stringify(
            PLATFORMS.flatMap(({ target, runner, signer }) =>
                signer === undefined ? [] : [{ target, runner, signer }],
            ),
        ),
        unsigned: globTargets(
            PLATFORMS.filter(({ signer }) => signer === undefined).map(({ target }) => target),
        ),
        installers: JSON.stringify(describeInstallers()),
        names: PLATFORMS.map(({ target }) => target).join(" "),
        renewals: JSON.stringify(renewals),
        checks: JSON.stringify(checks),
    };
    await writeOutputs(outputs);

    // report the selected work
    print(
        `Destack ${release.version} from ${commit}: build ${outputs.build}, publish ${outputs.publish}, renew ${outputs.renewals}, check ${outputs.checks}`,
    );
}

/** Select the stable release of the pushed tag or a unique calendar nightly of the run. */
function selectRelease(channel: Channel, isTag: boolean): Release {
    // require the run identity a nightly version derives from
    const sequence = process.env["GITHUB_RUN_NUMBER"];
    const attempt = process.env["GITHUB_RUN_ATTEMPT"];
    if (
        sequence === undefined ||
        attempt === undefined ||
        !/^[1-9]\d*$/u.test(sequence) ||
        !/^[1-9]\d*$/u.test(attempt)
    ) {
        throw new Error("release preparation requires a GitHub run number and attempt");
    }

    // retain the stable tag or derive a unique calendar nightly version
    const tag = process.env["GITHUB_REF_NAME"];
    const now = new Date();
    const version = isTag
        ? tag?.replace(/^v/u, "")
        : `${now.getUTCFullYear()}.${now.getUTCMonth() + 1}.${sequence}-nightly.${attempt}`;
    const release = new Release(version, "aarch64-apple-darwin");
    if (release.channel !== channel || (isTag && tag !== `v${release.version}`)) {
        throw new Error("release tag must select a stable calendar version");
    }

    return release;
}

/** Describe each installer's build: its format, name, runner, signer and the artifacts it reads. */
function describeInstallers(): Record<string, string>[] {
    return listInstallers().map(({ installer, platforms }) => {
        // require one signer across the applications an installer ships
        const signers = new Set(platforms.map(({ signer }) => signer));
        const [signer] = signers;
        if (signers.size !== 1 || signer === undefined) {
            throw new Error(`the ${installer.name} installer requires one signer`);
        }

        return {
            format: installer.format,
            name: installer.name,
            runner: installer.runner,
            signer,
            targets: globTargets(platforms.map(({ target }) => target)),
        };
    });
}

/** Match any of the targets with a glob, which expands braces only around two or more choices. */
function globTargets(targets: readonly string[]): string {
    const [target] = targets;

    return targets.length === 1 && target !== undefined ? target : `{${targets.join(",")}}`;
}

/** Read every channel's published repository, absent before its first publication. */
async function readPublished(): Promise<Map<Channel, RepositoryRenewal>> {
    // keep the channels with a published repository
    const published = new Map<Channel, RepositoryRenewal>();
    for (const name of CHANNELS) {
        const repository = new RepositoryConfiguration(name);
        const renewal = await RepositoryRenewal.read(repository.url, await repository.root());
        if (renewal !== undefined) {
            published.set(name, renewal);
        }
    }

    return published;
}

/** Append the job outputs to the `GITHUB_OUTPUT` file. */
async function writeOutputs(outputs: Record<string, string>): Promise<void> {
    // require the output file of the workflow step
    const output = process.env["GITHUB_OUTPUT"];
    if (output === undefined || output === "") {
        throw new Error("release preparation requires GITHUB_OUTPUT");
    }

    // write one name=value line per output
    await appendFile(
        output,
        Object.entries(outputs)
            .map(([name, value]) => `${name}=${value}\n`)
            .join(""),
    );
}

await prepare();
