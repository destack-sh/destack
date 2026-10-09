import { eventService } from "@destack/event/service";
import { defineView } from "@destack/view/declare";
import { alert, alertRule, issue } from "../object/index.ts";
import { observabilityService } from "../service/index.ts";

/** A space's recent log records, searched with the field query language and followed live. */
export const logs = defineView({
    name: "logs",
    services: [eventService],
    component: () => import("./log.tsx"),
});

/** A space's recent traces, and one trace's spans as a tree on a time line. */
export const traces = defineView({
    name: "traces",
    services: [eventService, observabilityService],
    component: () => import("./trace.tsx"),
});

/** A space's issues by status, and one issue with its events and lifecycle. */
export const issues = defineView({
    name: "issues",
    objects: [issue],
    permissions: { space: [issue.permission("read"), issue.permission("edit")] },
    services: [eventService],
    component: () => import("./issue.tsx"),
});

/** A space's alert rules with their evaluation, and the alerts they fired. */
export const alerts = defineView({
    name: "alerts",
    objects: [alertRule, alert],
    permissions: {
        space: [alertRule.permission("read"), alert.permission("read"), alert.permission("edit")],
    },
    component: () => import("./alert.tsx"),
});
