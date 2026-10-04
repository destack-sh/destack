import type { JSX } from "@solidjs/web";
import { Badge } from "./badge.tsx";

/** Show a note's state and its number of comments. */
export function BadgeExample(): JSX.Element {
    return (
        <p>
            Groceries <Badge variant="secondary">Draft</Badge> <Badge>3 comments</Badge>
        </p>
    );
}
