import { UNIVERSE_PAGES } from "@destack/account/address";
import { HOME_LOCATION_PATH, HomeLocation } from "@destack/account/authentication";
import { t } from "@destack/locale";
import { createSignal, onCleanup, onSettled, Show, useEnvironment, useLocale } from "@destack/view";
import { CardPage } from "./page.tsx";
import { callIssuer, createIssuerStep } from "./issuer.ts";
import { Failure } from "@destack/error";

/** How often a starting home is asked again: a home starts in one to two seconds locally. */
const STARTING_POLL_MILLISECONDS = 1000;

/** Open the signed-in person's Home, or the sign-in for a signed-out browser. */
export default function StartPage() {
    // locate the person's Home, at once and again each second while it starts, until the page closes
    const environment = useEnvironment();
    const locale = useLocale();
    const [isStarting, setStarting] = createSignal(false);
    const { error, run } = createIssuerStep();
    const closing = new AbortController();
    onCleanup(() => closing.abort());
    const open = () =>
        run(async () => {
            // read where Home answers, sending a signed-out browser to sign in
            let home: HomeLocation;
            try {
                home = HomeLocation.parse(await callIssuer(HOME_LOCATION_PATH, environment));
            } catch (refused: unknown) {
                if (Failure.of(refused).code === "UNAUTHORIZED") {
                    environment.navigation.assign(UNIVERSE_PAGES.signIn);

                    return;
                }
                throw refused;
            }

            // open a served Home, else say it is starting and ask again
            if (home.status === "serving") {
                environment.navigation.assign(home.url);
            } else {
                setStarting(true);
                void environment.clock
                    .sleep(STARTING_POLL_MILLISECONDS, closing.signal)
                    .then(open, () => undefined);
            }
        });
    onSettled(open);

    return (
        <CardPage title={locale.render(t`Opening your Home`)} error={error()}>
            <Show when={isStarting()}>
                <p>{locale.render(t`Your home is starting.`)}</p>
            </Show>
        </CardPage>
    );
}
