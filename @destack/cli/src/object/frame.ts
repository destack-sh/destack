import {
    BoardLayout,
    type Layout,
    ObjectAppearance,
    ObjectDetail,
    type ObjectType,
    ObjectValue,
    Presentation,
    TableLayout,
    type TimeWindow,
} from "@destack/object";
import { TABLE } from "@destack/db";
import { type JsonObject, present, schema } from "@destack/schema";
import type { Context } from "../context/context.ts";
import type { Terminal } from "../output/index.ts";
import type { Noun } from "./catalog.ts";
import { Cell } from "./cell.ts";
import { Grid } from "./grid.ts";
import { Page, type PageLine } from "./page.ts";
import { BoardPage } from "./layout/board.ts";
import { CalendarPage } from "./layout/calendar.ts";
import { GalleryPage } from "./layout/gallery.ts";
import { TablePage } from "./layout/table.ts";
import { TimelinePage } from "./layout/timeline.ts";
import { TreePage } from "./layout/tree.ts";

/** The margin a drawing starts at, beside the gutter's words. */
const MARGIN = "  ";

/** The space between two columns. */
const GAP = "  ";

/** The width a drawing takes when the terminal reports none: 80 columns, the VT100's. */
const DEFAULT_COLUMNS = 80;

/** The mark an object's accent draws before its title. */
const ACCENT_DOT = "●";

/** The narrowest board column, below which a board draws its columns as groups. */
const BOARD_COLUMN = 24;

/** The narrowest an object line's label shrinks to before its fields. */
const MINIMUM_LABEL = 12;

/** The narrowest terminal a calendar draws its month beside its agenda on. */
const SIDE_BY_SIDE = 60;

/** The indent of an object under its group's heading. */
const GROUPED = "  ";

/** The terminal a frame draws on: its styles, place, messages and width. */
export type Surface = Pick<
    Terminal,
    | "style"
    | "tone"
    | "swatch"
    | "heading"
    | "isTerminal"
    | "timeZone"
    | "locale"
    | "localization"
    | "columns"
>;

/** The objects some rows reference, by reference field, then identifier. */
export type References = ReadonlyMap<
    string,
    { readonly type: ObjectType; readonly rows: ReadonlyMap<string, JsonObject> }
>;

/** A row placed on a page: its depth and the guide drawn before it. */
interface Placed {
    /** The object. */
    readonly row: JsonObject;
    /** The depth below the page's roots. */
    readonly depth: number;
    /** The guide or indent before the line. */
    readonly guide: string;
}

/** One drawing: the terminal, the moment, and the objects the rows reference. */
export class Frame {
    /** The terminal drawn on. */
    readonly terminal: Surface;
    /** The moment drawn at, which near times read relative to and highlights compare with. */
    readonly now: number;
    /** The objects the rows reference, which a person reads by title, none for a program. */
    readonly references: References;

    /** Create a frame of a terminal, a moment and the referenced objects. */
    constructor(terminal: Surface, now: number, references: References) {
        this.terminal = terminal;
        this.now = now;
        this.references = references;
    }

    /** Open a frame for some rows, with the objects they reference for a person. */
    static async of(context: Context, noun: Noun, rows: readonly JsonObject[]): Promise<Frame> {
        // read no references for a program, which reads identifiers
        const { terminal } = context;
        const now = context.environment.clock.now();
        const references = new Map<
            string,
            { readonly type: ObjectType; readonly rows: ReadonlyMap<string, JsonObject> }
        >();
        if (!terminal.isTerminal) {
            return new Frame(terminal, now, references);
        }

        // read each reference field's objects through their type's noun
        const catalog = await context.catalog();
        for (const [column, field] of Object.entries(noun.type.fields)) {
            const target = field.target?.();
            const found = catalog.nouns.find(
                (each) => target !== undefined && each.type.same(target),
            );
            const ids = [
                ...new Set(
                    rows.flatMap((row) => (typeof row[column] === "string" ? [row[column]] : [])),
                ),
            ];
            if (found !== undefined && ids.length > 0) {
                const remote = await found.client(context);
                const objects = await remote.list(found.type, { where: { id: { in: ids } } });
                const byId = new Map(
                    objects.map((object) => [schema.string().parse(object["id"]), object]),
                );
                references.set(column, { type: found.type, rows: byId });
            }
        }

        return new Frame(terminal, now, references);
    }

    /** Read the field titling a type's objects, the identifier for a type titling none. */
    static title(type: ObjectType): string {
        return type.presentation?.title ?? "id";
    }

    /** Read a row's title as text, else its identifier. */
    static titleText(type: ObjectType, row: JsonObject): string {
        return Presentation.title(type.presentation, row);
    }

    /** Write a key the cursor and folds follow: text as it is, any other value as JSON. */
    static key(value: unknown): string {
        return typeof value === "string" ? value : JSON.stringify(value);
    }

    /** Read the columns a table shows: its layout's, else the title, first fields and id. */
    static columns(type: ObjectType, layout: TableLayout): readonly string[] {
        // take the layout's columns as named, else the title, the first shown fields and the identifier
        const title = type.presentation?.title;
        const shown = TableLayout.columns({ kind: "table" }, ObjectDetail.shown(type));

        return (
            layout.fields ??
            [...(title === undefined ? [] : [title]), ...shown, "id"].filter((column) =>
                Object.hasOwn(type.table[TABLE].columns, column),
            )
        );
    }

    /** Write a row's columns as cells, the title marked by its accent, references titled. */
    cells(type: ObjectType, row: JsonObject, columns: readonly string[]): Cell[] {
        // read the row's accent at the frame's moment
        const { terminal } = this;
        const moment = { now: this.now, timeZone: terminal.timeZone };
        const { color } = ObjectAppearance.of(type, row, moment);

        return columns.map((column) => {
            // write the identifier people use, dim, and a program's id whole
            if (column === "id") {
                const id = terminal.isTerminal ? this.identifier(type, row) : Frame.key(row["id"]);

                return { ...Cell.plain(id), paint: (text: string) => terminal.style("dim", text) };
            }

            // write a reference as its object's title, any other value by its meaning
            const value = row[column];
            const referenced = this.references.get(column);
            const object = typeof value === "string" ? referenced?.rows.get(value) : undefined;
            const cell =
                referenced === undefined || object === undefined
                    ? Cell.of(ObjectValue.of(type, column, value), terminal, this.now)
                    : Cell.plain(ObjectAppearance.of(referenced.type, object, moment).title);

            // mark the title with the object's accent
            return column === type.presentation?.title && color !== undefined
                ? {
                      ...cell,
                      text: `${ACCENT_DOT} ${cell.text}`,
                      paint: (text: string) =>
                          `${terminal.swatch(color, text.slice(0, 1))}${cell.paint(text.slice(1))}`,
                  }
                : cell;
        });
    }

    /** Head a column by its field's label, else its name, a reference by its relation. */
    heading(type: ObjectType, column: string): string {
        const field = type.fields[column];
        const name = field?.reference === undefined ? column : field.relationAt(column);

        return field?.label === undefined ? name : this.terminal.localization.render(field.label);
    }

    /** Read the identifier people use for a row, such as TRK-214. */
    identifier(type: ObjectType, row: JsonObject): string {
        // read the key of the object the row's number counts within, when it counts within one
        const within = type.numbering?.within;
        const value = within === undefined ? undefined : row[within];
        const holder = within === undefined ? undefined : this.references.get(within);
        const owner = typeof value === "string" ? holder?.rows.get(value) : undefined;
        const keyed = holder?.type.keyed;
        const key =
            owner === undefined || keyed === undefined
                ? undefined
                : schema.string().parse(owner[keyed]);

        return ObjectAppearance.identifier(type, row, key);
    }

    /** Draw rows as a table: a header, one row a line and their count, else tab-separated rows. */
    table(rows: readonly JsonObject[], columns: readonly string[], type: ObjectType): string {
        // tell a person there are none, and a program nothing
        const { terminal } = this;
        if (rows.length === 0) {
            return terminal.isTerminal
                ? `${MARGIN}${terminal.style("dim", `no ${type.plural}`)}`
                : "";
        }

        // draw each row's cells under the columns' headings, the count below for a person
        const grid = Grid.draw(
            columns.map((column) => this.heading(type, column)),
            rows.map((row) => this.cells(type, row, columns)),
            terminal,
        );

        return terminal.isTerminal ? `${grid}\n\n${this.counted(rows, type)}` : grid;
    }

    /** Draw one object as its fields, one a line, under its title for a person. */
    record(row: JsonObject, type: ObjectType): string {
        // head a person's record with the object's title and identifier, its other fields below
        const { terminal } = this;
        const title = type.presentation?.title;
        const headed = terminal.isTerminal && title !== undefined && typeof row[title] === "string";
        const fields = Object.keys(row).filter(
            (field) => !headed || (field !== title && field !== "id"),
        );
        const width = Math.max(...fields.map((field) => Cell.width(this.heading(type, field))));
        const head = headed
            ? [
                  `${MARGIN}${terminal.heading(schema.string().parse(row[title]), this.identifier(type, row))}`,
                  "",
              ]
            : [];
        const cells = this.cells(type, row, fields);

        return [
            ...head,
            ...fields.map((field, index) => {
                // write the cell beside its field's heading for a person, tab-separated for a program
                const cell = cells[index] ?? Cell.plain("");

                return terminal.isTerminal
                    ? `${MARGIN}${terminal.style("dim", this.heading(type, field).padEnd(width))}  ${cell.paint(cell.text)}`
                    : `${field}\t${cell.text}`;
            }),
        ].join("\n");
    }

    /** Draw rows by a layout: records for a program, a drawing or the page for a person. */
    draw(
        rows: readonly JsonObject[],
        layout: Layout,
        type: ObjectType,
        window?: TimeWindow,
    ): string {
        // write records for a program, and tell a person when there are none
        const { terminal } = this;
        if (!terminal.isTerminal) {
            return this.records(rows, layout, type, window);
        } else if (rows.length === 0) {
            return `${MARGIN}${terminal.style("dim", `no ${type.plural}`)}`;
        }

        // draw a board's columns, a calendar's month and a gallery's cards beside each other when they fit
        const columns = terminal.columns ?? DEFAULT_COLUMNS;
        const groups = layout.kind === "board" ? BoardLayout.group(layout, rows, type.fields) : [];
        const width =
            Math.floor((columns - MARGIN.length) / Math.max(1, groups.length)) - GAP.length;
        if (layout.kind === "board" && width >= BOARD_COLUMN) {
            return BoardPage.draw(this, groups, type, width, layout.by);
        } else if (layout.kind === "calendar" && columns >= SIDE_BY_SIDE) {
            return CalendarPage.draw(this, this.page(rows, layout, type, window));
        } else if (layout.kind === "gallery") {
            return GalleryPage.draw(this, rows, type);
        }

        return this.page(rows, layout, type, window).joined();
    }

    /** Write rows as tab-separated records for a program: each object's place and title. */
    records(
        rows: readonly JsonObject[],
        layout: Layout,
        type: ObjectType,
        window?: TimeWindow,
    ): string {
        // write a table by its columns, any other layout as its kind's records
        if (layout.kind === "table") {
            return this.table(rows, Frame.columns(type, layout), type);
        }
        const records =
            layout.kind === "tree"
                ? TreePage.records(rows, layout, type)
                : layout.kind === "timeline"
                  ? TimelinePage.records(rows, layout, type)
                  : layout.kind === "board"
                    ? BoardPage.records(rows, layout, type)
                    : layout.kind === "calendar"
                      ? CalendarPage.records(this, rows, layout, type, window)
                      : GalleryPage.records(rows, type);

        return records.map((record) => record.join("\t")).join("\n");
    }

    /** Draw a layout as a page: a header, its lines of objects and groups, and a footer. */
    page(rows: readonly JsonObject[], layout: Layout, type: ObjectType, window?: TimeWindow): Page {
        // draw the layout's kind
        switch (layout.kind) {
            case "table":
                return TablePage.of(this, rows, layout, type);
            case "tree":
                return TreePage.of(this, rows, layout, type);
            case "board":
                return BoardPage.of(this, rows, layout, type);
            case "calendar":
                return CalendarPage.of(this, rows, layout, type, window);
            case "timeline":
                return TimelinePage.of(this, rows, layout, type);
            case "gallery":
                return GalleryPage.of(this, rows, type);
        }
    }

    /** Draw groups of rows as a page: each group's heading over its objects' lines. */
    grouped(
        groups: readonly {
            readonly key: string;
            readonly heading: string;
            readonly rows: readonly JsonObject[];
        }[],
        type: ObjectType,
        exclude: readonly string[],
        header: readonly string[],
        rows: readonly JsonObject[],
    ): Page {
        // draw each grouped row's line, then each group's heading over its lines
        const placed = groups.flatMap((group) =>
            group.rows.map((row) => ({ row, depth: 1, guide: GROUPED })),
        );
        const byRow = new Map(
            this.lines(placed, type, exclude, "id").map((line) => [line.row, line]),
        );
        const lines = groups.flatMap((group) => [
            {
                key: `group:${group.key}`,
                depth: 0,
                row: undefined,
                text: `${MARGIN}${this.terminal.style("bold", group.heading)}`,
            },
            ...group.rows.map((row) => present(byRow.get(row), "the line of a grouped row")),
        ]);

        return new Page(header, lines, this.counted(rows, type));
    }

    /** Count rows below a drawing, dim. */
    counted(rows: readonly JsonObject[], type: ObjectType): string {
        return `${MARGIN}${this.terminal.style("dim", `${rows.length} ${rows.length === 1 ? type.name : type.plural}`)}`;
    }

    /** Draw placed rows as object lines: guide, label, key fields and identifier. */
    lines(
        placed: readonly Placed[],
        type: ObjectType,
        exclude: readonly (string | undefined)[],
        key: string,
    ): PageLine[] {
        // write each row's label, key fields and identifier as cells
        const columns = [
            Frame.title(type),
            ...ObjectDetail.keys(type).filter((field) => !exclude.includes(field)),
            "id",
        ];
        const cells = placed.map(({ row }) => this.cells(type, row, columns));

        // fit the labels into what the fields leave of the width
        const widths = columns
            .slice(1)
            .map((_, index) =>
                Math.max(...cells.map((line) => Cell.width(line[index + 1]?.text ?? ""))),
            );
        const fields = widths.reduce((sum, width) => sum + width + GAP.length, 0);
        const room = (this.terminal.columns ?? DEFAULT_COLUMNS) - MARGIN.length - fields;
        const natural = Math.max(
            ...placed.map(
                (each, index) => Cell.width(each.guide) + Cell.width(cells[index]?.[0]?.text ?? ""),
            ),
        );
        const labelWidth = Math.max(MINIMUM_LABEL, Math.min(room, natural));

        return placed.map(({ row, depth, guide }, index) => {
            // fit the label beside its guide, and each field to its column
            const [label = Cell.plain(""), ...values] = cells[index] ?? [];
            const fitted = Cell.fit(label, Math.max(1, labelWidth - Cell.width(guide)));
            const aligned = values
                .map((value, each) => `${GAP}${Cell.fit(value, widths[each] ?? 0)}`)
                .join("");

            return {
                key: Frame.key(row[key]),
                depth,
                row,
                text: `${MARGIN}${this.terminal.style("dim", guide)}${fitted}${aligned}`.trimEnd(),
            };
        });
    }

    /** Draw a card: the object's title over its key fields, dim, each fitted to a width. */
    card(row: JsonObject, type: ObjectType, width: number, exclude: readonly string[]): string[] {
        // write the title, and the key fields beside the excluded ones as one dim line
        const keys = ObjectDetail.keys(type).filter((field) => !exclude.includes(field));
        const [label = Cell.plain(""), ...fields] = this.cells(type, row, [
            Frame.title(type),
            ...keys,
        ]);
        const detail = {
            ...Cell.plain(fields.map((each) => each.text).join(" · ")),
            paint: (text: string) => this.terminal.style("dim", text),
        };

        return [Cell.fit(label, width), Cell.fit(detail, width)];
    }
}
