import { defineExample } from "@destack/package/declare";
import { Verification } from "./verification.tsx";

/** A television's sign-in waiting for the person's decision. */
export const verificationPending = defineExample({
    of: Verification,
    name: "pending",
    description: "a television's sign-in waiting for the person's decision",
    render: () => (
        <Verification
            request={{ userCode: "WTTC-PPVX", client: "Destack TV", status: "pending" }}
            onSubmit={() => undefined}
            onApprove={() => undefined}
            onDeny={() => undefined}
        />
    ),
});
