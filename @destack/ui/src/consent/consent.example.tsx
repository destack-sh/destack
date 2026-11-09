import { defineExample } from "@destack/package/declare";
import { Consent, ConsentScope } from "./consent.tsx";

/** The command line asking to sign in and stay signed in. */
export const consentCommandLine = defineExample({
    of: Consent,
    name: "command-line",
    description: "the command line asking to sign in and stay signed in",
    render: () => (
        <Consent onApprove={() => undefined} onDeny={() => undefined}>
            <ConsentScope description="Know who you are" />
            <ConsentScope description="Stay signed in" />
        </Consent>
    ),
});
