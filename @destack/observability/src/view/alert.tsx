import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import {
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemTitle,
    itemVariants,
} from "@destack/ui/item";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { For, Loading, useQuery, useSpace } from "@destack/view";
import { alert, type Alert, alertRule, type AlertRule } from "../object/index.ts";
import { PAGE_ROWS, styles, timeOf } from "./page.tsx";

/** List a space's alert rules with their evaluation beside the alerts they fired, newest first, resolving firing ones by hand. */
export default function Alerts() {
    // follow the rules by name and the alerts newest first
    const objects = useSpace({ alertRule, alert });
    const rules = useQuery(
        objects.query.alertRule.findMany({ orderBy: { name: "asc" }, limit: PAGE_ROWS }),
    );
    const alerts = useQuery(
        objects.query.alert.findMany({ orderBy: { createdAt: "desc" }, limit: PAGE_ROWS }),
    );

    return (
        <main {...style.attrs(text.footnote, styles.page)} aria-label="Alerts">
            {/* Rules beside alerts */}
            <div {...style.attrs(styles.split)}>
                <Loading fallback={<p>Loading alert rules</p>}>
                    <RuleList rules={rules()} />
                </Loading>
                <Loading fallback={<p>Loading alerts</p>}>
                    <AlertList
                        alerts={alerts()}
                        resolve={(id) => void objects.mutate.alert.resolve({ id }).confirmed}
                    />
                </Loading>
            </div>
        </main>
    );
}

/** List alert rules with what fires them and whether their controller evaluates them. */
function RuleList(properties: {
    /** The rules, by name. */
    rules: readonly Pick<AlertRule, "name" | "condition" | "conditions">[];
}) {
    return (
        <ol {...style.attrs(styles.list)} aria-label="Alert rules">
            <For each={properties.rules}>
                {(rule) => (
                    <li
                        data-slot="rule"
                        {...style.attrs(itemVariants({ variant: "outline", size: "sm" }))}
                    >
                        <ItemContent>
                            <ItemTitle>{rule.name}</ItemTitle>
                            <ItemDescription>{conditionOf(rule)}</ItemDescription>
                        </ItemContent>
                        <Badge
                            variant={
                                rule.conditions["Ready"]?.status === "false"
                                    ? "destructive"
                                    : "secondary"
                            }
                        >
                            {rule.conditions["Ready"]?.reason ?? "Pending"}
                        </Badge>
                    </li>
                )}
            </For>
        </ol>
    );
}

/** List alerts with their status, resolving a firing one by hand. */
function AlertList(properties: {
    /** The alerts, newest first. */
    alerts: readonly Pick<Alert, "id" | "title" | "status" | "createdAt">[];
    /** Resolve a firing alert. */
    resolve: (id: Alert["id"]) => void;
}) {
    return (
        <ol {...style.attrs(styles.list)} aria-label="Alerts">
            <For each={properties.alerts}>
                {(row) => (
                    <li
                        data-slot="alert"
                        {...style.attrs(itemVariants({ variant: "outline", size: "sm" }))}
                    >
                        <ItemContent>
                            <ItemTitle>
                                {row.title}
                                <Badge
                                    variant={row.status === "firing" ? "destructive" : "secondary"}
                                >
                                    {row.status}
                                </Badge>
                            </ItemTitle>
                            <ItemDescription>{timeOf(row.createdAt)}</ItemDescription>
                        </ItemContent>
                        <ItemActions>
                            <Button
                                size="sm"
                                variant="outline"
                                disabled={row.status !== "firing"}
                                onClick={() => properties.resolve(row.id)}
                            >
                                Resolve
                            </Button>
                        </ItemActions>
                    </li>
                )}
            </For>
        </ol>
    );
}

/** Describe what fires a rule as people read it. */
function conditionOf(rule: Pick<AlertRule, "condition">): string {
    const { condition } = rule;

    // an issue condition
    if (condition.kind === "issue") {
        const where = condition.filter === undefined ? "" : ` where ${condition.filter}`;

        return `When an issue ${condition.on}s${where}`;
    }
    // an events condition
    else {
        const measured = condition.measure === undefined ? "" : ` of ${condition.measure}`;

        return `When the ${condition.fold}${measured} of ${condition.event} events is ${condition.comparison} ${condition.threshold}`;
    }
}
