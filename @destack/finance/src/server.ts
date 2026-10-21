import accountPackage from "@destack/account/package";
import forgePackage from "@destack/forge/package";
import { Registry } from "@destack/forge/client";
import { defineServer } from "@destack/object/server";
import { createClient } from "@destack/object/client";
import { provider, tag } from "./declare/setting.ts";
import financePackage from "./package.ts";
import { StripeProvider } from "./provider/stripe/provider.ts";
import { stripeSecretKey } from "./provider/stripe/secret.ts";
import { Metering } from "./server/metering.ts";
import { implementFinance } from "./server/server.ts";

export * from "./server/index.ts";

/** The address of the account package, which the universe serves. */
const ACCOUNT_ADDRESS = accountPackage.name;

/** The finance package's server in a residency home: sellers, prices, subscriptions and usage. */
export default defineServer(financePackage, (context) => {
    // charge buyers through the installation's provider, none for default products alone
    const home = context.space;
    const charging =
        provider.read(context.settings) === "none"
            ? undefined
            : new StripeProvider({
                  key: context.resources.get(stripeSecretKey),
                  home,
                  fetch: (request) => context.fetch(request),
              });

    // find each handle's seller to the home's buyers through the universe's residencies
    const accounts = context.at(ACCOUNT_ADDRESS);
    const client = createClient(accountPackage, {
        url: accounts.url,
        fetch: (request) => accounts.fetch(request),
    });
    const publisher = (handle: string) => client.residency.publisher({ handle, home });

    // read the packages' published builds from the forge in the home
    const forge = context.at(forgePackage.name);
    const registry = new Registry(async () => ({
        url: forge.url,
        space: home,
        fetch: forge.fetch,
    }));
    const metering = new Metering(tag.read(context.settings), publisher, (packageId, manifest) =>
        registry.read(packageId, manifest),
    );

    return implementFinance(home, metering, charging);
});
