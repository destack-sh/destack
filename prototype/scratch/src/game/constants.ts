// NOTE @Architecture: centralizes tunable game constants
// Increase speeds/time slightly for a snappier feel.

export const VIRTUAL_WIDTH = 1200;
export const VIRTUAL_HEIGHT = 900;

export const PADDLE_BASE_WIDTH = 160;
export const PADDLE_HEIGHT = 16;
export const PADDLE_SPEED_PX_PER_S = 700;
export const PADDLE_MAX_WIDTH = 260;
export const PADDLE_MIN_WIDTH = 80;

export const BALL_RADIUS = 8;
export const BALL_START_SPEED_PX_PER_S = 400;
export const BALL_MAX_SPEED_PX_PER_S = 880;
export const BALL_SPEED_INCREMENT_ON_HIT = 12;

export const BRICK_COLS = 24;
export const BRICK_ROWS = 16;
export const BRICK_GUTTER = 6;
export const BRICK_TOP_MARGIN = 80;
export const BRICK_SIDE_MARGIN = 16;
export const BRICK_HEIGHT = 20;
export const BRICK_BOTTOM_MARGIN = 200; // keep space for paddle/gameplay

export const MAX_LIVES = 5;
export const START_LIVES = 3;

export const POWERUP_DROP_CHANCE = 0.15; // 15%
export const POWERUP_DURATION_MS = 12000;

export const FRAME_DT_CAP_S = 1 / 30; // cap delta time for stability
export const FIXED_DT_S = 1 / 120; // fixed simulation timestep (Hz)
export const TIME_SCALE = 1.1; // simulate a bit faster than real-time
export const MAX_STEPS_PER_FRAME = 14; // safety cap per frame

export const STORAGE_KEY = "breakout_progress_v1";

export const COLORS = {
  background: "#0f0f14",
  foreground: "#e6e6e6",
  paddle: "#9ae6b4",
  ball: "#fcd34d",
  brick1: "#60a5fa",
  brick2: "#34d399",
  brick3: "#f472b6",
  brick4: "#fbbf24",
  brick5: "#f87171",
  uiGood: "#22c55e",
  uiWarn: "#f59e0b",
  uiBad: "#ef4444",
} as const;


