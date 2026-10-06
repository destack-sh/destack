import { defineAlertRule } from "@destack/observability/declare";

/** Roll an installation back to its previous revision when a new fatal issue opens. */
export const rollback = defineAlertRule({
    name: "Roll back new fatal issues",
    condition: { kind: "issue", on: "open", filter: "level = fatal" },
    actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
});
