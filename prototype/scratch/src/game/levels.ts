import {
    BRICK_COLS,
    BRICK_GUTTER,
    BRICK_HEIGHT,
    BRICK_SIDE_MARGIN,
    BRICK_TOP_MARGIN,
    BRICK_BOTTOM_MARGIN,
    COLORS,
    VIRTUAL_WIDTH,
    VIRTUAL_HEIGHT,
} from "./constants";
import type { Brick } from "./types";

// NOTE @Architecture: define a few handcrafted levels; can be extended with parser later
export const LEVELS: Array<{ pattern: string[] }> = [
    {
        pattern: [
            "............",
            "..########..",
            "..########..",
            "..########..",
            "..########..",
            "............",
            "..########..",
            "..########..",
        ],
    },
    {
        pattern: [
            "############",
            "#..##..##..#",
            "############",
            "#..##..##..#",
            "############",
            "#..##..##..#",
            "############",
            "#..##..##..#",
        ],
    },
    {
        pattern: [
            "#.#.#.#.#.#.",
            ".#.#.#.#.#.#",
            "#.#.#.#.#.#.",
            ".#.#.#.#.#.#",
            "#.#.#.#.#.#.",
            ".#.#.#.#.#.#",
            "#.#.#.#.#.#.",
            ".#.#.#.#.#.#",
        ],
    },
];

export function createLevelBricks(levelIndex: number): Brick[] {
    const level = LEVELS[levelIndex % LEVELS.length]!;
    const cols = BRICK_COLS;
    const brickWidth = Math.floor(
        (VIRTUAL_WIDTH - BRICK_SIDE_MARGIN * 2 - BRICK_GUTTER * (cols - 1)) / cols,
    );

    const bricks: Brick[] = [];
    const patternRows = level.pattern.length;
    // compute rows to fill screen height reasonably
    const usableHeight = Math.max(0, VIRTUAL_HEIGHT - BRICK_TOP_MARGIN - BRICK_BOTTOM_MARGIN);
    const rows = Math.floor((usableHeight + BRICK_GUTTER) / (BRICK_HEIGHT + BRICK_GUTTER));
    for (let r = 0; r < rows; r++) {
        const rowPattern = level.pattern[r % patternRows]!;
        const patternCols = rowPattern.length;
        for (let c = 0; c < cols; c++) {
            const ch = rowPattern[c % patternCols]!;
            if (ch === ".") continue;
            const x = BRICK_SIDE_MARGIN + c * (brickWidth + BRICK_GUTTER);
            const y = BRICK_TOP_MARGIN + r * (BRICK_HEIGHT + BRICK_GUTTER);
            const colorIndex = (r % 5) + 1;
            const hp = ch === "@" ? 2 : 1;
            bricks.push({
                x,
                y,
                w: brickWidth,
                h: BRICK_HEIGHT,
                alive: true,
                hp,
                colorIndex,
                score: 50 + r * 5,
            });
        }
    }
    return bricks;
}

export function colorForBrick(colorIndex: number): string {
    const map = [COLORS.brick1, COLORS.brick2, COLORS.brick3, COLORS.brick4, COLORS.brick5];
    return map[(colorIndex - 1) % map.length]!;
}
