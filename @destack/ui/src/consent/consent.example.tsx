import { defineExample } from "@destack/package/declare";
import {
    ConsentPrompt,
    Grant,
    GrantList,
    Permission,
    PermissionGroup,
    PermissionList,
} from "./consent.tsx";

/** The command line asking to know who the person is and stay signed in. */
export const consentCommandLine = defineExample({
    of: ConsentPrompt,
    name: "command-line",
    description: "the command line asking to know who the person is and stay signed in",
    render: () => (
        <ConsentPrompt onAllow={() => undefined} onDeny={() => undefined}>
            <PermissionList>
                <PermissionGroup title="Your account">
                    <Permission description="Know who you are" />
                    <Permission description="Stay signed in" />
                </PermissionGroup>
            </PermissionList>
        </ConsentPrompt>
    ),
});

/** An app asking for access beyond its space, one permission allowed before. */
export const consentApp = defineExample({
    of: ConsentPrompt,
    name: "app",
    description: "an app asking for access beyond its space, one permission allowed before",
    render: () => (
        <ConsentPrompt
            onAllow={() => undefined}
            onAllowOnce={() => undefined}
            onDeny={() => undefined}
        >
            <PermissionList>
                <PermissionGroup title="In your home">
                    <Permission description="See settings" isGranted />
                    <Permission description="See calendar events" />
                </PermissionGroup>
                <PermissionGroup title="In your account">
                    <Permission description="See spaces" />
                </PermissionGroup>
            </PermissionList>
        </ConsentPrompt>
    ),
});

/** Two apps a person lets act for them. */
export const grantApps = defineExample({
    of: GrantList,
    name: "apps",
    description: "two apps a person lets act for them",
    render: () => (
        <GrantList>
            <Grant name="Notes" description="In work--ada" onRemove={() => undefined}>
                <PermissionList>
                    <PermissionGroup title="In your home">
                        <Permission description="See settings" />
                    </PermissionGroup>
                </PermissionList>
            </Grant>
            <Grant name="Tasks" description="In team--acme" onRemove={() => undefined} />
        </GrantList>
    ),
});
