import { present } from "@destack/schema";
import { Show } from "@destack/view";
import type { RouteSectionProps } from "@destack/view/router";
import { Metadata } from "@destack/view/document";

import { policyContent, policies } from "../content/collection";
import { PolicyArticle } from "../reader/policy";
import { MissingPage } from "./missing";

/** Render one policy, or the missing page for an unknown slug. */
export function Policy(properties: RouteSectionProps) {
    // find the policy and its body by the slug
    const slug = () => present(properties.params["slug"], "a policy slug");
    const page = () => policies.find((each) => each.slug === slug());
    const loaded = () => policyContent.get(slug());

    return (
        <Show
            keyed
            when={page()}
            fallback={
                <MissingPage
                    backHref="/"
                    backLabel="back home"
                    description="This policy does not exist."
                    label="missing page"
                    title="this page does not exist"
                />
            }
        >
            {(found) => (
                <>
                    <Metadata
                        title={found.title}
                        description={found.description}
                        alternates={{
                            canonical: found.route,
                            types: {
                                "text/markdown": found.markdownRoute,
                                "text/plain": found.textRoute,
                            },
                        }}
                    />
                    <Show when={loaded()}>
                        {(body) => <PolicyArticle content={body()} page={found} />}
                    </Show>
                </>
            )}
        </Show>
    );
}
