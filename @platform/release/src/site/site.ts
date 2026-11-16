import { join, resolve } from "node:path";
import { CompilerPool, Workspace } from "@destack/build";
import { Toolchain } from "@destack/check/toolchain";
import type { Environment, HostCapability } from "@destack/environment";
import { StaticExport } from "@destack/machine";
import { DistributionManifest } from "@destack/package";
import type { Lock } from "@destack/package/manifest";
import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { type Digest, found } from "@destack/schema";
import { PlatformBuild } from "../deployment/build.ts";
import { ACCOUNT_ID, CloudflareCredentials } from "../deployment/cloudflare/credential.ts";
import { ROOT, Root, WRANGLER_VARIABLES } from "../deployment/root.ts";
import { CHECKOUT } from "../distribution/distribution.ts";

/** The site's package, by its directory below the checkout's root. */
const SITE = "@platform/site";

/** The site's static view, which its package declares. */
const VIEW = "site";

/** The Worker serving the site's files at destack.sh. */
const WORKER = "destack-site";

/** The runtime whose linked builtins the pages import, as cloud machines link them. */
const PLATFORM = "workerd";

/** Build the site with the platform's releases, export its static view and deploy its Worker. */
export async function deploySite(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // refuse arguments, as the site has one deployment
    if (arguments_.length > 0) {
        throw new Error(`site deploy takes nothing more: '${arguments_.join(" ")}'`);
    }

    // build the platform's releases, and the site's lock in the same store
    await using platform = await PlatformBuild.build(CHECKOUT, environment);
    const { store } = platform;
    const lock = await siteLock(platform);

    // read the builtins a cloud machine links, at their locks, by package name as a machine links them
    const builtins = DistributionManifest.builtins(platform.distribution(PLATFORM), PLATFORM);
    const linked = new Map<string, Lock>();
    for (const release of builtins.values()) {
        linked.set(release.package.name, await store.lock(release.lock));
    }

    // export the site's view into a fresh directory, as a machine's origin serves it
    const directory = resolve(ROOT, ".wrangler", "site");
    const assets = join(directory, "public");
    await environment.files.remove(directory, { recursive: true, force: true });
    await new StaticExport(assets, store, environment).write(lock, VIEW, linked);

    // deploy the files as the site's Worker, which serves its pages as the site's host configuration has them
    const configuration = {
        name: WORKER,
        account_id: ACCOUNT_ID,
        compatibility_date: WORKERD_COMPATIBILITY_DATE,
        assets: {
            directory: "public",
            html_handling: "auto-trailing-slash",
            not_found_handling: "404-page",
        },
    };
    const path = join(directory, "wrangler.json");
    await environment.files.writeTextFile(path, `${JSON.stringify(configuration, null, 4)}\n`);
    await Root.execute(
        resolve(ROOT, "node_modules/.bin/wrangler"),
        ["deploy", "--config", path],
        {
            ...Root.inherited(environment.process.variables, WRANGLER_VARIABLES),
            ...(await CloudflareCredentials.resolve(environment)).environment,
            WRANGLER_SEND_METRICS: "false",
        },
        environment,
    );
}

/** Build the site's package in the platform's store, answering its lock. */
async function siteLock(platform: PlatformBuild): Promise<Digest> {
    // build and lock the site's units with the platform's compilers
    await using compilers = await CompilerPool.open(platform.store, async () =>
        Toolchain.workspace(),
    );
    const workspace = await Workspace.open(CHECKOUT, compilers);
    const site = await workspace.package(join(CHECKOUT, SITE));
    const units = await site.lockUnits({});

    return found(units, SITE).lock;
}
