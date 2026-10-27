import forgePackage from "@destack/forge/package";
import { Registry } from "@destack/forge/client";
import { implementPackage } from "@destack/object/server";
import { provider, tag } from "./declare/setting.ts";
import financePackage from "./package.ts";
import { StripeProvider } from "./provider/stripe/provider.ts";
import { stripeSecretKey } from "./provider/stripe/secret.ts";
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
            : new StripeProvider({
                  key: context.resources.get(stripeSecretKey),
                  home,
                  fetch: (request) => context.fetch(request),
              });

    // read the packages' published builds from the forge in the home
    const forge = context.at(forgePackage.name);
    const registry = new Registry(
        async () => ({ url: forge.url, space: home, fetch: forge.fetch }),
        context.environment,
    );
    const metering = new Metering(
        tag.read(context.settings),
        (packageId, manifest) => registry.read(packageId, manifest),
        context.environment.telemetry,
    );

    return implementFinance(home, metering, charging);
});
