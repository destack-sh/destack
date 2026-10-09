import { defineExample } from "@destack/package/declare";
import { Badge } from "../badge/index.ts";
import { Button } from "../button/index.ts";
import { Cluster } from "./cluster.tsx";

/** A note's tags in a row that wraps at narrow widths. */
export const clusterTags = defineExample({
    of: Cluster,
    name: "tags",
    description: "a note's tags in a row that wraps at narrow widths",
    render: () => (
        <Cluster space="2" render={(attributes) => <ul aria-label="Tags" {...attributes} />}>
            {["launch", "planning", "marketing", "q4", "waitlist"].map((tag) => (
                <li>
                    <Badge variant="secondary">{tag}</Badge>
                </li>
            ))}
        </Cluster>
    ),
});

/** A toolbar with a title on one end and actions on the other. */
export const clusterToolbar = defineExample({
    of: Cluster,
    name: "toolbar",
    description: "a toolbar with a title on one end and actions on the other",
    render: () => (
        <Cluster justify="between">
            <h2>Notes</h2>
            <Cluster space="2">
                <Button variant="ghost">Import</Button>
                <Button>New note</Button>
            </Cluster>
        </Cluster>
    ),
});
