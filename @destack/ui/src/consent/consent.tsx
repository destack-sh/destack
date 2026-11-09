import { t } from "@destack/locale";
import { type JSX, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Stack } from "../stack/index.ts";

/** The properties of a consent: the scopes an application asks for and the person's answer. */
export interface ConsentProperties {
    /** Whether a decision is on its way, which disables both buttons. */
    readonly isPending?: boolean | undefined;
    /** Allow the application what it asks. */
    readonly onApprove: () => void;
    /** Deny it. */
    readonly onDeny: () => void;
    /** The scopes the application asks for, one `ConsentScope` each. */
    readonly children?: JSX.Element;
}

/** Render the scopes an application asks for, then allow or deny them. */
export function Consent(properties: ConsentProperties): JSX.Element {
    const locale = useLocale();
    const isPending = () => properties.isPending === true;

    return (
        <Stack space="4">
            <ul>{properties.children}</ul>
            <Stack space="2">
                <Button disabled={isPending()} onClick={() => properties.onApprove()}>
                    {locale.render(t`Allow`)}
                </Button>
                <Button
                    variant="outline"
                    disabled={isPending()}
                    onClick={() => properties.onDeny()}
                >
                    {locale.render(t`Deny`)}
                </Button>
            </Stack>
        </Stack>
    );
}

/** The properties of one scope an application asks for. */
export interface ConsentScopeProperties {
    /** What the scope lets the application do, as the person reads it. */
    readonly description: string;
}

/** Render one scope an application asks for. */
export function ConsentScope(properties: ConsentScopeProperties): JSX.Element {
    return <li>{properties.description}</li>;
}
