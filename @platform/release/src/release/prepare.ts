import type { Environment, HostCapability } from "@destack/environment";
import { Release } from "@destack/update/release";
import { run } from "../distribution/command.ts";
import { print } from "../distribution/output.ts";
import { CHECKOUT, readCommit } from "../distribution/distribution.ts";
import { Installer, PLATFORMS } from "../distribution/platform.ts";
import { RepositoryConfiguration } from "../repository/configuration.ts";
import { RepositoryRenewal } from "../repository/renewal.ts";

/** Select a release run's version and work before building or exposing signing credentials. */
export async function prepareRelease(environment: Environment<HostCapability>): Promise<void> {
    // publish only from main, reading what the feed published, which versions follow
    const configuration = new RepositoryConfiguration(environment);
    const isPublishing = environment.process.variables["GITHUB_REF"] === "refs/heads/main";
    if (isPublishing) {
        const ancestry = ["merge-base", "--is-ancestor", "HEAD", "origin/main"];
        await run("git", ancestry, environment, CHECKOUT);
    }
    const published = await RepositoryRenewal.read(
        configuration.url,
        configuration.root(),
        environment,
    );
    const release = selectRelease(published?.version, environment);

    // build when main moved past the published release's commit, else renew what is published
    const commit = await readCommit(CHECKOUT, environment);
    const isBuilding = published === undefined || !published.isBuiltFrom(commit);
    const isPublishingBuild = isPublishing && isBuilding;
    const isRenewing = published !== undefined && !isBuilding;

    // expose identical public build choices to every job
    const outputs = {
        version: release.version,
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
        renew: String(isRenewing),
        check: String(isRenewing || isPublishingBuild),
    };
    await writeOutputs(outputs, environment);

    // report the selected work
    print(
        `Destack ${release.version} from ${commit}: build ${outputs.build}, publish ${outputs.publish}, renew ${outputs.renew}, check ${outputs.check}`,
        environment,
    );
}

/** Select the release after the feed's published one in its month, else this month's first. */
function selectRelease(published: string | undefined, environment: Environment): Release {
    // read this month and the published release's month and number
    const now = new Date(environment.clock.now());
    const month = `${now.getUTCFullYear()}.${now.getUTCMonth() + 1}`;
    const matched =
        published === undefined ? null : /^(\d{4}\.\d+)\.(0|[1-9]\d*)$/u.exec(published);

    // take the number after the published one in its month, else start the month at 0
    const next = matched?.[1] === month ? Number(matched[2]) + 1 : 0;

    return new Release(`${month}.${next}`, "aarch64-apple-darwin");
}

/** Describe each installer's build: its format, name, runner, signer and the artifacts it reads. */
function describeInstallers(): Record<string, string>[] {
    return Installer.list().map(({ installer, platforms }) => {
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

/** Append the job outputs to the `GITHUB_OUTPUT` file. */
async function writeOutputs(
    outputs: Record<string, string>,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // require the output file of the workflow step
    const output = environment.process.variables["GITHUB_OUTPUT"];
    if (output === undefined || output === "") {
        throw new Error("release preparation requires GITHUB_OUTPUT");
    }

    // write one name=value line per output
    await environment.files.writeTextFile(
        output,
        Object.entries(outputs)
            .map(([name, value]) => `${name}=${value}\n`)
            .join(""),
        { append: true },
    );
}
