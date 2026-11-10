import { defineExample } from "@destack/package/declare";
import { ConsentPrompt, Permission, PermissionGroup, PermissionList } from "../consent/index.ts";
import { Verification } from "./verification.tsx";

/** A television's sign-in waiting for the person's consent. */
export const verificationPending = defineExample({
    of: Verification,
    name: "pending",
    description: "a television's sign-in waiting for the person's consent",
    render: () => (
        <Verification
            request={{ userCode: "WTTC-PPVX", client: "Destack TV", status: "pending" }}
            onSubmit={() => undefined}
        >
            <ConsentPrompt onAllow={() => undefined} onDeny={() => undefined}>
                <PermissionList>
                    <PermissionGroup title="Your account">
                        <Permission description="Know who you are" />
                        <Permission description="Stay signed in" />
                    </PermissionGroup>
                </PermissionList>
            </ConsentPrompt>
        </Verification>
    ),
});
