import type { Event } from "@destack/event";
import { Badge } from "@destack/ui/badge";
import { Switch } from "@destack/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@destack/ui/table";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { createSignal, For, Loading, useView } from "@destack/view";
import { levelOf, log } from "../event/index.ts";
import { useEvents } from "./feed.ts";
import { FilterField, MoreButton, styles, timeOf, useFilter } from "./page.tsx";

/** The microseconds in a millisecond, as events time themselves in microseconds. */
const MICROSECONDS = 1000;

/** Search a space's log records with the field query language, newest first, following new ones while live. */
export default function Logs() {
    // follow the records the filter selects while live
    const view = useView();
    const filter = useFilter();
    const [isLive, setLive] = createSignal(true);
    const records = useEvents(() => {
        const where = filter.where();

        return where === undefined
            ? undefined
            : { kind: "log", scope: view.space, ...(where === "" ? {} : { where }) };
    }, isLive);

    return (
        <main {...style.attrs(text.footnote, styles.page)} aria-label="Logs">
            {/* Filter and live switch */}
            <header {...style.attrs(styles.header)}>
                <FilterField filter={filter} placeholder="severity >= 17 AND name : note" />
                <label {...style.attrs(styles.header)}>
                    <Switch checked={isLive()} onCheckedChange={setLive} />
                    Live
                </label>
            </header>

            {/* Records */}
            <Loading fallback={<p>Loading logs</p>}>
                <RecordTable records={records().events} />
                <MoreButton more={records().more} />
            </Loading>
        </main>
    );
}

/** Show log records as a table of their time, level, name and body. */
function RecordTable(properties: {
    /** The log events, newest first. */
    records: readonly Event[];
}) {
    return (
        <Table aria-label="Records">
            {/* Columns */}
            <TableHeader>
                <TableRow>
                    <TableHead>Time</TableHead>
                    <TableHead>Level</TableHead>
                    <TableHead>Name</TableHead>
                    <TableHead>Body</TableHead>
                </TableRow>
            </TableHeader>

            {/* Rows */}
            <TableBody>
                <For each={properties.records}>{(event) => <RecordRow event={event} />}</For>
            </TableBody>
        </Table>
    );
}

/** Show one log record as a table row. */
function RecordRow(properties: {
    /** The log event. */
    event: Event;
}) {
    // read the record's keys and body
    const keys = () => log.parseKeys(properties.event.keys);
    const level = () => levelOf(keys().severity);
    const isFailure = () => level() === "error" || level() === "fatal";

    return (
        <TableRow data-slot="log">
            <TableCell>{timeOf(properties.event.time / MICROSECONDS)}</TableCell>
            <TableCell>
                <Badge variant={isFailure() ? "destructive" : "secondary"}>
                    {level() ?? "unset"}
                </Badge>
            </TableCell>
            <TableCell>{keys().name}</TableCell>
            <TableCell>{log.parseData(properties.event.data).body ?? ""}</TableCell>
        </TableRow>
    );
}
