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
import { type Localization, t } from "@destack/locale";
import { For, Loading, useLocale, useQuery, useSpace } from "@destack/view";
import { alert, type Alert, alertRule, type AlertRule } from "../object/index.ts";
import { PAGE_ROWS, styles, Time } from "./page.tsx";

/** List a space's alert rules with their evaluation beside the alerts they fired, newest first, resolving firing ones by hand. */
export default function Alerts() {
    // follow the rules by title and the alerts newest first
    const locale = useLocale();
    const objects = useSpace({ alertRule, alert });
    const rules = useQuery(
        objects.query.alertRule.findMany({ orderBy: { title: "asc" }, limit: PAGE_ROWS }),
    );
    const alerts = useQuery(
        objects.query.alert.findMany({ orderBy: { createdAt: "desc" }, limit: PAGE_ROWS }),
    );

    return (
        <main {...style.attrs(text.footnote, styles.page)} aria-label={locale.render(t`Alerts`)}>
            {/* Rules beside alerts */}
            <div {...style.attrs(styles.split)}>
                <Loading fallback={<p>{locale.render(t`Loading alert rules`)}</p>}>
                    <RuleList rules={rules()} />
                </Loading>
                <Loading fallback={<p>{locale.render(t`Loading alerts`)}</p>}>
                    <AlertList
                        alerts={alerts()}
                        resolve={(id) => void objects.mutate.alert.resolve({ id }).confirmed}
                    />
                </Loading>
            </div>
        </main>
    );
}

/** List alert rules with what fires them and whether their controller evaluated them yet. */
function RuleList(properties: {
    /** The rules, by title. */
    rules: readonly Pick<AlertRule, "title" | "condition" | "conditions">[];
}) {
    const locale = useLocale();

    return (
        <ol {...style.attrs(styles.list)} aria-label={locale.render(t`Alert rules`)}>
            <For each={properties.rules}>
                {(rule) => (
                    <li
                        data-slot="rule"
                        {...style.attrs(itemVariants({ variant: "outline", size: "sm" }))}
                    >
                        <ItemContent>
                            <ItemTitle>{rule.title}</ItemTitle>
                            <ItemDescription>{conditionOf(rule, locale)}</ItemDescription>
                        </ItemContent>
                        <Badge variant="secondary">
                            {rule.conditions["Ready"] === undefined
                                ? locale.render(t`Pending`)
                                : locale.render(t`Evaluated`)}
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
    const locale = useLocale();

    return (
        <ol {...style.attrs(styles.list)} aria-label={locale.render(t`Alerts`)}>
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
                            <ItemDescription>
                                <Time at={row.createdAt} />
                            </ItemDescription>
                        </ItemContent>
                        <ItemActions>
                            <Button
                                size="sm"
                                variant="outline"
                                disabled={row.status !== "firing"}
                                onClick={() => properties.resolve(row.id)}
                            >
                                {locale.render(t`Resolve`)}
                            </Button>
                        </ItemActions>
                    </li>
                )}
            </For>
        </ol>
    );
}

/** Describe what fires a rule as people read it, in their locale. */
function conditionOf(rule: Pick<AlertRule, "condition">, locale: Localization): string {
    const { condition } = rule;

    // an issue condition, over the issues a filter selects when it has one
    if (condition.kind === "issue") {
        const changed = {
            open: t`When an issue opens`,
            regress: t`When an issue regresses`,
            escalate: t`When an issue escalates`,
        }[condition.on];
        const described = locale.render(changed);

        return condition.filter === undefined
            ? described
            : locale.render(t`${described} where ${condition.filter}`);
    }
    // an events condition, of a measured key when it folds one
    else {
        const threshold = locale.number(condition.threshold);
        const folded =
            condition.measure === undefined
                ? condition.fold
                : locale.render(t`${condition.fold} of ${condition.measure}`);

        return condition.comparison === "above"
            ? locale.render(
                  t`When the ${folded} of ${condition.event} events is above ${threshold}`,
              )
            : locale.render(
                  t`When the ${folded} of ${condition.event} events is below ${threshold}`,
              );
    }
}
