import type { JSX } from "@destack/view";

import type { Entry } from "../figure/item";
import { Goodreads } from "../personal/goodreads";
import { Letterboxd } from "../personal/letterboxd";
import { Monzo } from "../personal/monzo";
import { Steam } from "../personal/steam";
import { Strava } from "../personal/strava";

/** The bill row's label for a personal app, whichever app it shows: an app outside your space. */
const EXTERNAL = "External";

/** A personal app your agent connects to your space, its data synced into a table of yours. */
export type Source = {
    /** The app's name. */
    name: string;
    /** The app's mark in the diagram icons. */
    icon: string;
    /** The app's brand colour. */
    tint: string;
    /** The address of your page in the app. */
    site: string;
    /** What of yours the app keeps, as a plural noun. */
    data: string;
    /** The table of your space the data syncs into. */
    table: string;
    /** The table's three column heads. */
    columns: readonly [string, string, string];
    /** The latest rows, newest first. */
    rows: readonly (readonly [string, string, string])[];
    /** Draw your page in the app at the size of a real window. */
    page: () => JSX.Element;
};

/** The personal apps the hero's data row turns through, each with a public API or export. */
export const sources: readonly Source[] = [
    {
        name: "Strava",
        icon: "strava",
        tint: "#fc4c02",
        site: "strava.com/dashboard",
        data: "runs",
        table: "Workouts",
        columns: ["Date", "Run", "Distance"],
        rows: [
            ["Oct 21", "Morning run", "8.2 km"],
            ["Oct 19", "Long run", "21.1 km"],
            ["Oct 17", "Intervals", "6.4 km"],
        ],
        page: Strava,
    },
    {
        name: "Monzo",
        icon: "monzo",
        tint: "#ff4f40",
        site: "web.monzo.com",
        data: "payments",
        table: "Budget",
        columns: ["Date", "Payee", "Amount"],
        rows: [
            ["Oct 21", "Hetzner", "£16.40"],
            ["Oct 20", "Workshop Coffee", "£3.80"],
            ["Oct 18", "Trainline", "£42.10"],
        ],
        page: Monzo,
    },
    {
        name: "Goodreads",
        icon: "goodreads",
        tint: "#553b08",
        site: "goodreads.com/user/show/1984-florian",
        data: "books",
        table: "Reading",
        columns: ["Finished", "Book", "Rating"],
        rows: [
            ["Oct 16", "The Dispossessed", "★★★★★"],
            ["Sep 30", "Piranesi", "★★★★"],
            ["Sep 12", "The Mythical Man-Month", "★★★★"],
        ],
        page: Goodreads,
    },
    {
        name: "Steam",
        icon: "steam",
        tint: "#1b2838",
        site: "steamcommunity.com/id/florian",
        data: "games",
        table: "Games",
        columns: ["Played", "Game", "Hours"],
        rows: [
            ["Oct 20", "Factorio", "1,204 h"],
            ["Oct 18", "Tiny Glade", "31 h"],
            ["Oct 12", "Outer Wilds", "27 h"],
        ],
        page: Steam,
    },
    {
        name: "Letterboxd",
        icon: "letterboxd",
        tint: "#00a35c",
        site: "letterboxd.com/florian/diary",
        data: "films",
        table: "Watched",
        columns: ["Watched", "Film", "Rating"],
        rows: [
            ["Oct 19", "Perfect Days", "★★★★½"],
            ["Oct 12", "Past Lives", "★★★★"],
            ["Oct 4", "Stalker", "★★★★★"],
        ],
        page: Letterboxd,
    },
];

/** Return a source's bill row: its data kept in the app, or synced into your tables. */
export function entryOf(source: Source): Entry {
    return {
        stacked: {
            label: EXTERNAL,
            mark: { icon: source.icon, tint: source.tint },
            name: source.name,
            price: { kind: "bundled", basis: `kept in ${source.name}` },
        },
        destacked: {
            label: EXTERNAL,
            mark: { icon: source.icon, tint: source.tint },
            name: source.name,
            price: { kind: "included" },
        },
    };
}
