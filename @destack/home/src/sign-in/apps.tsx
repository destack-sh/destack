import { CONSENT_PATH, type Consent, ConsentList } from "@destack/account/authentication";
import { argument, t } from "@destack/locale";
import type { schema } from "@destack/schema";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import { Grant, GrantList } from "@destack/ui/consent";
import { createSignal, For, onSettled, Show, useEnvironment, useLocale } from "@destack/view";
import { GrantPermissions } from "./grant.tsx";
import { CardPage } from "./page.tsx";
import { callIssuer, createIssuerStep } from "./issuer.ts";

/** List the apps the person lets act for them, with what each may do, and remove one. */
export default function AppsPage() {
    // read the person's consents, sending a person signed out to sign in
    const environment = useEnvironment();
    const locale = useLocale();
    const [consents, setConsents] = createSignal<schema.Infer<typeof Consent>[]>();
    const { error, isPending, run } = createIssuerStep();
    const list = async () => {
        const listed = ConsentList.parse(await callIssuer(CONSENT_PATH, environment));
        setConsents(listed.consents);
    };
    onSettled(() => run(list));

    // describe where an app acts and since when
    const describe = (consent: schema.Infer<typeof Consent>) => {
        const since = locale.date(consent.grantedAt, { dateStyle: "medium" });

        // say since when an app without a grant was allowed
        if (consent.grant === null) {
            return locale.render(t`Allowed ${argument("date")}`, { date: since });
        }
        // say a grant for this sign-in alone lasts until it ends
        else if (consent.grant.lifetime === "sign-in") {
            return locale.render(
                t`${argument("package")} in ${argument("space")}, until you sign out`,
                { package: consent.grant.package, space: consent.grant.space },
            );
        }
        // say since when a lasting grant was allowed
        else {
            return locale.render(
                t`${argument("package")} in ${argument("space")}, allowed ${argument("date")}`,
                { package: consent.grant.package, space: consent.grant.space, date: since },
            );
        }
    };

    // remove an app's access, then list the rest
    const remove = (clientId: string) =>
        run(async () => {
            // remove its consent and grant at the issuer
            const query = new URLSearchParams({ client_id: clientId });
            await callIssuer(`${CONSENT_PATH}?${query}`, environment, undefined, "DELETE");

            // list the rest
            await list();
        });

    return (
        <CardPage
            title={locale.render(t`Apps`)}
            description={locale.render(t`Apps that can act for you, and what you allowed them.`)}
            error={error()}
        >
            <Show when={consents()}>
                {(listed) => (
                    <Show
                        when={listed().length > 0}
                        fallback={
                            <Empty>
                                <EmptyHeader>
                                    <EmptyTitle>{locale.render(t`No apps`)}</EmptyTitle>
                                    <EmptyDescription>
                                        {locale.render(t`Apps you allow appear here.`)}
                                    </EmptyDescription>
                                </EmptyHeader>
                            </Empty>
                        }
                    >
                        <GrantList>
                            <For each={listed()}>
                                {(consent) => (
                                    <Grant
                                        name={consent.client.name}
                                        description={describe(consent)}
                                        isPending={isPending()}
                                        onRemove={() => remove(consent.client.id)}
                                    >
                                        <GrantPermissions
                                            grant={consent.grant}
                                            scopes={consent.scopes}
                                        />
                                    </Grant>
                                )}
                            </For>
                        </GrantList>
                    </Show>
                )}
            </Show>
        </CardPage>
    );
}
