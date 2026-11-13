import { defineMount, definePackage } from "@destack/object";
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
    respond,
    RESPOND_CAPABILITY,
    session,
    silence,
    span,
    visit,
    visitorSalt,
} from "./object/index.ts";
import { t } from "@destack/locale";

/** The handle spaces serve issues, alerts, telemetry and analytics through. */
export default definePackage({
    title: t`Observability`,
    description: t`Query, follow and alert on the telemetry and analytics of Destack scopes, group their failures into issues, and measure what their spaces use.`,
    icon: "pulse",
    color: "violet",
    objects: [
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
        session,
        visit,
        action,
        ...respond.objects,
    ],
    mounts: [defineMount({ name: "otlp", path: "/v1" })],
    access: {
        permissions: {
            space: {
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
    },
    capabilities: [
        {
            name: RESPOND_CAPABILITY,
            reason: t`Roll back installations when an alert rule you write fires`,
            optional: true,
            permissions: {
                space: { space: [rollout.permission("apply")] },
                user: { space: [rollout.permission("apply")] },
            },
        },
    ],
    errors: ObservabilityError,
});
