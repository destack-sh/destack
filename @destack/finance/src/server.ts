import forgePackage from "@destack/forge/package";
import { RemoteBuildStore } from "@destack/build/store";
import type { PackageId } from "@destack/package";
import registryPackage from "@destack/registry/package";
import { createClient } from "@destack/object/client";
import { implementPackage } from "@destack/object/server";
import { provider, tag } from "./declare/setting.ts";
import financePackage from "./package.ts";
import { StripeProvider } from "./provider/stripe/provider.ts";
import { stripe } from "./declare/connection.ts";
import { Metering } from "./server/metering.ts";
import { implementFinance } from "./server/server.ts";

export * from "./server/index.ts";

/** The finance package's server in a residency home: sellers, prices, subscriptions and usage. */
export default implementPackage(financePackage, (context) => {
    // charge buyers through the installation's provider, none for default products alone
    const home = context.scope;
    const charging =
        provider.read(context.settings) === "none"
            ? undefined
            : new StripeProvider({ api: context.resources.get(stripe), home });

    // read the packages' published builds from the forge in the home
    const forge = context.at(forgePackage.name);
    const registry = createClient(registryPackage, forge, context.environment);
    const builds = `${forge.url}${registryPackage.mounts.builds.path}`;
    const open = async (packageId: PackageId, named: string) => {
        const found = await registry.release.resolve({
            spaceId: home,
            parentId: packageId,
            release: named,
        });

        return await RemoteBuildStore.open(
            builds,
            forge.fetch,
            found.image,
            context.environment.crypto,
        );
    };
    const metering = new Metering(tag.read(context.settings), open, context.environment.telemetry);

    return implementFinance(home, metering, charging, context);
});
