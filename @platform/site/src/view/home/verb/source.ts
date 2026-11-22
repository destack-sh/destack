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
    /** The latest rows, newest first, each from its day of the story. */
    rows: readonly (readonly [day: number, string, string])[];
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
            [0, "Morning run", "8.2 km"],
            [-1, "Long run", "21.1 km"],
            [-3, "Intervals", "6.4 km"],
            [-5, "Easy run", "7.0 km"],
            [-7, "Hill repeats", "9.3 km"],
            [-8, "Long run", "18.4 km"],
            [-10, "Recovery run", "5.1 km"],
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
            [0, "Hetzner", "£16.40"],
            [0, "Workshop Coffee", "£3.80"],
            [-2, "Trainline", "£42.10"],
            [-3, "Notion", "£96.00"],
            [-4, "Waitrose", "£38.65"],
            [-5, "Figma", "£45.00"],
            [-6, "Pret", "£6.20"],
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
            [-4, "The Dispossessed", "★★★★★"],
            [-20, "Piranesi", "★★★★"],
            [-38, "The Mythical Man-Month", "★★★★"],
            [-52, "Klara and the Sun", "★★★★"],
            [-70, "A Psalm for the Wild-Built", "★★★★★"],
            [-88, "The Design of Everyday Things", "★★★"],
            [-110, "Exhalation", "★★★★★"],
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
            [0, "Factorio", "1,204 h"],
            [-2, "Tiny Glade", "31 h"],
            [-8, "Outer Wilds", "27 h"],
            [-15, "Balatro", "96 h"],
            [-22, "Hades II", "54 h"],
            [-29, "Shapez 2", "212 h"],
            [-36, "Return of the Obra Dinn", "11 h"],
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
            [-1, "Perfect Days", "★★★★½"],
            [-8, "Past Lives", "★★★★"],
            [-16, "Stalker", "★★★★★"],
            [-23, "Aftersun", "★★★★½"],
            [-30, "The Zone of Interest", "★★★★"],
            [-37, "Paterson", "★★★★"],
            [-44, "Columbus", "★★★½"],
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
