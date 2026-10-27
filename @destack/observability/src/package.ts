import { defineAdapter, definePackage } from "@destack/object";
import { rollout } from "@destack/stack/object";
import { ObservabilityError } from "./error/error.ts";
import {
    action,
    alert,
    alertRule,
    issue,
    issuePerson,
    exception,
    log,
    metric,
    reading,
    respond,
    RESPOND_CAPABILITY,
    session,
    silence,
    span,
    visit,
    visitorSalt,
} from "./object/index.ts";

/** The handle spaces serve issues, alerts, telemetry and analytics through. */
export default definePackage({
    objects: {
        issue,
        issuePerson,
        exception,
        alertRule,
        alert,
        silence,
        visitorSalt,
        log,
        span,
        metric,
        reading,
        session,
        visit,
        action,
        ...respond.objects,
    },
    adapters: { otlp: defineAdapter("otlp", { path: "/v1" }) },
    permissions: {
        installation: {
            // group issues, evaluate alert rules, raise and fire alerts, and announce both
            space: [
                issue.permission("read"),
                issue.permission("detect"),
                alertRule.permission("read"),
                alertRule.permission("control"),
                alert.permission("detect"),
            ],
        },
    },
    capabilities: {
        // run alert rules' calls, such as rolling back an installation, as the people who write them
        [RESPOND_CAPABILITY]: {
            reason: "Roll back installations when an alert rule you write fires",
            optional: true,
            permissions: {
                installation: { space: [rollout.permission("apply")] },
                user: { space: [rollout.permission("apply")] },
            },
        },
    },
    errors: ObservabilityError,
});
