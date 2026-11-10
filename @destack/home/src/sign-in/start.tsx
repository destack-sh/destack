import { UNIVERSE_PAGES } from "@destack/account/address";
import { HOME_LOCATION_PATH, HomeLocation } from "@destack/account/authentication";
import { t } from "@destack/locale";
import { Button } from "@destack/ui/button";
import { createSignal, onSettled, Show, useEnvironment, useLocale } from "@destack/view";
import { CardPage } from "./page.tsx";
import { callIssuer, createIssuerStep } from "./issuer.ts";
import { Failure } from "@destack/error";

/** Open the signed-in person's Home, or the sign-in for a signed-out browser. */
export default function StartPage() {
    // locate the person's Home, at once and again on the person's click
    const environment = useEnvironment();
    const locale = useLocale();
    const [isStarting, setStarting] = createSignal(false);
    const { error, isPending, run } = createIssuerStep();
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

            // open a served Home, else say it is starting
            if (home.status === "serving") {
                environment.navigation.assign(home.url);
            } else {
                setStarting(true);
            }
        });
    onSettled(open);

    return (
        <CardPage title={locale.render(t`Opening your Home`)} error={error()}>
            <Show when={isStarting()}>
                <p>{locale.render(t`Your home is starting.`)}</p>
                <Button disabled={isPending()} onClick={open}>
                    {locale.render(t`Open it`)}
                </Button>
            </Show>
        </CardPage>
    );
}
