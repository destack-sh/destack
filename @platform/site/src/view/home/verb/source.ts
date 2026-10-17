import type { Entry } from "../figure/item";

/** A personal app your agent connects to your space, its data synced into a table of yours. */
export type Source = {
    /** The job the app does, set as the bill row's label. */
    label: string;
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
};

/** The personal apps the hero's data row turns through, each with a public API or export. */
export const sources: readonly Source[] = [
    {
        label: "Fitness",
        name: "Strava",
        icon: "strava",
        tint: "#fc4c02",
        site: "strava.com/athletes/florian",
        data: "runs",
        table: "Workouts",
        columns: ["Date", "Run", "Distance"],
        rows: [
            ["Oct 21", "Morning run", "8.2 km"],
            ["Oct 19", "Long run", "21.1 km"],
            ["Oct 17", "Intervals", "6.4 km"],
        ],
    },
    {
        label: "Banking",
        name: "Monzo",
        icon: "monzo",
        tint: "#ff4f40",
        site: "web.monzo.com/feed",
        data: "payments",
        table: "Budget",
        columns: ["Date", "Payee", "Amount"],
        rows: [
            ["Oct 21", "Hetzner", "£16.40"],
            ["Oct 20", "Workshop Coffee", "£3.80"],
            ["Oct 18", "Trainline", "£42.10"],
        ],
    },
    {
        label: "Reading",
        name: "Goodreads",
        icon: "goodreads",
        tint: "#553b08",
        site: "goodreads.com/review/list/florian",
        data: "books",
        table: "Reading",
        columns: ["Finished", "Book", "Rating"],
        rows: [
            ["Oct 16", "The Dispossessed", "★★★★★"],
            ["Sep 30", "Piranesi", "★★★★"],
            ["Sep 12", "The Mythical Man-Month", "★★★★"],
        ],
    },
    {
        label: "Health",
        name: "Fitbit",
        icon: "fitbit",
        tint: "#00b0b9",
        site: "fitbit.com/global/us/home",
        data: "nights",
        table: "Sleep",
        columns: ["Night", "Asleep", "Resting"],
        rows: [
            ["Oct 21", "7 h 12 m", "52 bpm"],
            ["Oct 20", "6 h 40 m", "55 bpm"],
            ["Oct 19", "8 h 05 m", "51 bpm"],
        ],
    },
    {
        label: "Games",
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
    },
    {
        label: "Films",
        name: "Letterboxd",
        icon: "letterboxd",
        tint: "#00a35c",
        site: "letterboxd.com/florian",
        data: "films",
        table: "Watched",
        columns: ["Watched", "Film", "Rating"],
        rows: [
            ["Oct 19", "Perfect Days", "★★★★½"],
            ["Oct 12", "Past Lives", "★★★★"],
            ["Oct 4", "Stalker", "★★★★★"],
        ],
    },
];

/** Return a source's bill row: its data kept in the app, or synced into your tables. */
export function entryOf(source: Source): Entry {
    return {
        stacked: {
            label: source.label,
            mark: { icon: source.icon, tint: source.tint },
            name: source.name,
            note: `your ${source.data}, kept in ${source.name}`,
            price: { kind: "bundled", basis: `kept in ${source.name}` },
        },
        destacked: {
            label: source.label,
            mark: { icon: source.icon, tint: source.tint },
            name: source.name,
            note: `your ${source.data}, synced into your ${source.table} table`,
            price: { kind: "included" },
        },
    };
}
