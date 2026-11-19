import { OriginPath } from "@destack/package";
import { onSettled, useEnvironment } from "@destack/view";
import { BrowserClient } from "@destack/account/browser";
import { Shell } from "./shell.tsx";

/** Show the account's spaces as Home's shell, binding it to this browser. */
export default function Home() {
    // bind the shell's credential to this browser's client once it shows
    const environment = useEnvironment();
    onSettled(() => {
        const { origin } = new URL(environment.navigation.currentEntry.url);
        new BrowserClient({ path: OriginPath.session, origin, environment })
            .bind(BrowserClient.describe(navigator))
            .catch((error: unknown) => globalThis.reportError(error));
    });

    return <Shell />;
}
