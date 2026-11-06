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
    /** The table's headline figures, each a label and its value. */
    figures: readonly (readonly [label: string, value: string])[];
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
        figures: [
            ["This week", "35.7 km"],
            ["Runs this year", "142"],
            ["Best half", "1:39:12"],
        ],
        rows: [
            ["Oct 21", "Morning run", "8.2 km"],
            ["Oct 19", "Long run", "21.1 km"],
            ["Oct 17", "Intervals", "6.4 km"],
            ["Oct 15", "Easy run", "7.0 km"],
            ["Oct 13", "Hill repeats", "9.3 km"],
            ["Oct 12", "Long run", "18.4 km"],
            ["Oct 10", "Recovery run", "5.1 km"],
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
        figures: [
            ["Spent this month", "£1,284"],
            ["Left to spend", "£716"],
            ["Subscriptions", "£248"],
        ],
        rows: [
            ["Oct 21", "Hetzner", "£16.40"],
            ["Oct 20", "Workshop Coffee", "£3.80"],
            ["Oct 18", "Trainline", "£42.10"],
            ["Oct 17", "Notion", "£96.00"],
            ["Oct 16", "Waitrose", "£38.65"],
            ["Oct 15", "Figma", "£45.00"],
            ["Oct 14", "Pret", "£6.20"],
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
        figures: [
            ["Read this year", "23"],
            ["Reading goal", "30"],
            ["Pages", "7,412"],
        ],
        rows: [
            ["Oct 16", "The Dispossessed", "★★★★★"],
            ["Sep 30", "Piranesi", "★★★★"],
            ["Sep 12", "The Mythical Man-Month", "★★★★"],
            ["Aug 29", "Klara and the Sun", "★★★★"],
            ["Aug 11", "A Psalm for the Wild-Built", "★★★★★"],
            ["Jul 24", "The Design of Everyday Things", "★★★"],
            ["Jul 2", "Exhalation", "★★★★★"],
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
        figures: [
            ["Played this week", "9 h"],
            ["Games", "84"],
            ["Unplayed", "31"],
        ],
        rows: [
            ["Oct 20", "Factorio", "1,204 h"],
            ["Oct 18", "Tiny Glade", "31 h"],
            ["Oct 12", "Outer Wilds", "27 h"],
            ["Oct 5", "Balatro", "96 h"],
            ["Sep 28", "Hades II", "54 h"],
            ["Sep 21", "Shapez 2", "212 h"],
            ["Sep 14", "Return of the Obra Dinn", "11 h"],
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
        figures: [
            ["Films this year", "61"],
            ["This month", "4"],
            ["Average rating", "3.9"],
        ],
        rows: [
            ["Oct 19", "Perfect Days", "★★★★½"],
            ["Oct 12", "Past Lives", "★★★★"],
            ["Oct 4", "Stalker", "★★★★★"],
            ["Sep 27", "Aftersun", "★★★★½"],
            ["Sep 20", "The Zone of Interest", "★★★★"],
            ["Sep 13", "Paterson", "★★★★"],
            ["Sep 6", "Columbus", "★★★½"],
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
