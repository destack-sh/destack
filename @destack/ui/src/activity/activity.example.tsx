import { t } from "@destack/locale";
import checkCircleIcon from "@destack/icon/phosphor/check-circle";
import tagIcon from "@destack/icon/phosphor/tag";
import userPlusIcon from "@destack/icon/phosphor/user-plus";
import { defineExample } from "@destack/package/declare";
import { useEnvironment } from "@destack/view";
import { Avatar, AvatarFallback, AvatarImage } from "../avatar/index.ts";
import { Badge } from "../badge/index.ts";
import { Card, CardContent } from "../card/index.ts";
import { PICTURES } from "../example/picture.ts";
import { Icon } from "../icon/index.ts";
import { RelativeTime } from "../relative-time/index.ts";
import { Activity, ActivityBadge, ActivityBody, ActivityItem } from "./activity.tsx";

/** The milliseconds of an hour. */
const HOUR = 3_600_000;

/** What happened to a task, from its first owner to its end. */
export const activityTask = defineExample({
    of: Activity,
    name: "task",
    description: t`What happened to a task: small changes, a comment as a card, and its end.`,
    layout: "padded",
    render: () => {
        const { clock } = useEnvironment();

        return (
            <Activity aria-label="Activity">
                <ActivityItem condensed>
                    <ActivityBadge>
                        <Icon icon={userPlusIcon} />
                    </ActivityBadge>
                    <ActivityBody>
                        <span>
                            Ada gave the task to Grace ·{" "}
                            <RelativeTime value={clock.now() - 50 * HOUR} />
                        </span>
                    </ActivityBody>
                </ActivityItem>
                <ActivityItem condensed>
                    <ActivityBadge>
                        <Icon icon={tagIcon} />
                    </ActivityBadge>
                    <ActivityBody>
                        <span>
                            Grace added <Badge color="blue">Launch</Badge> ·{" "}
                            <RelativeTime value={clock.now() - 26 * HOUR} />
                        </span>
                    </ActivityBody>
                </ActivityItem>
                <ActivityItem>
                    <ActivityBadge>
                        <Avatar>
                            <AvatarImage src={PICTURES.grace} alt="" />
                            <AvatarFallback>GH</AvatarFallback>
                        </Avatar>
                    </ActivityBadge>
                    <ActivityBody>
                        <Card>
                            <CardContent>
                                The prices are in. I left the annual plan out until we know what it
                                costs us.
                            </CardContent>
                        </Card>
                    </ActivityBody>
                </ActivityItem>
                <ActivityItem>
                    <ActivityBadge color="green">
                        <Icon icon={checkCircleIcon} />
                    </ActivityBadge>
                    <ActivityBody>
                        <span>
                            Grace finished the task · <RelativeTime value={clock.now() - HOUR} />
                        </span>
                    </ActivityBody>
                </ActivityItem>
            </Activity>
        );
    },
});
