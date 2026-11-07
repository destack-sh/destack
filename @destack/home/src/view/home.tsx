import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { space } from "@destack/theme/tokens.stylex";
import { Card, CardContent, CardHeader, CardTitle } from "@destack/ui/card";
import { t } from "@destack/locale";
import { Loading, onSettled, useEnvironment, useLocale } from "@destack/view";
import { CLIENT_PATH } from "@destack/view/declare";
import { BrowserClient } from "@destack/account/browser";
import { Spaces } from "./space.tsx";

/** The home layout: the account's spaces with the views their apps mount. */
const styles = style.create({
    page: {
        display: "grid",
        gap: space[5],
        maxWidth: "64rem",
        marginInline: "auto",
        padding: space[6],
    },
});

/** Show the account's spaces with their apps' views, binding the shell to this browser. */
export default function Home() {
    // read the page's localization and environment
    const locale = useLocale();
    const environment = useEnvironment();

    // bind the shell's credential to this browser's client once it shows
    onSettled(() => {
        const { origin } = new URL(environment.navigation.currentEntry.url);
        new BrowserClient({ path: CLIENT_PATH, origin, environment })
            .bind(BrowserClient.describe(navigator))
            .catch((error: unknown) => globalThis.reportError(error));
    });

    return (
        <main {...style.attrs(styles.page)}>
            {/* Title */}
            <h1 {...style.attrs(text.largeTitle)}>{locale.render(t`Home`)}</h1>

            {/* Spaces */}
            <Card>
                <CardHeader>
                    <CardTitle>{locale.render(t`Spaces`)}</CardTitle>
                </CardHeader>
                <CardContent>
                    <Loading>
                        <Spaces />
                    </Loading>
                </CardContent>
            </Card>
        </main>
    );
}
