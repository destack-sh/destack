export type Vector2 = {
  x: number;
  y: number;
};

export type Rect = {
  x: number;
  y: number;
  w: number;
  h: number;
};

export type Circle = {
  x: number;
  y: number;
  r: number;
};

export type Brick = Rect & {
  alive: boolean;
  hp: number;
  colorIndex: number;
  score: number;
};

export type PowerUpKind =
  | "widen"
  | "shrink"
  | "multiball"
  | "slow"
  | "fast"
  | "sticky"
  | "life";

export type ActivePowerUp = {
  kind: PowerUpKind;
  activeUntilMs: number;
};

export type FallingPowerUp = {
  kind: PowerUpKind;
  x: number;
  y: number;
  vy: number;
};

export type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  lifeMs: number;
  size: number;
  color: string;
};

export type Ball = Circle & {
  vx: number;
  vy: number;
  speed: number;
  stuckToPaddle: boolean;
};

export type Paddle = Rect & {
  vx: number;
  stickyUntilMs: number;
};

export type InputState = {
  left: boolean;
  right: boolean;
  launch: boolean;
  pauseToggle: boolean;
  rewind?: boolean;
};

export type GameState = {
  score: number;
  bestScore: number;
  lives: number;
  levelIndex: number;
  highestLevel: number;
  bricks: Brick[];
  paddle: Paddle;
  balls: Ball[];
  activePowerUps: ActivePowerUp[];
  fallingPowerUps: FallingPowerUp[];
  particles: Particle[];
  shakeMsRemaining: number;
  shakeMagnitude: number;
  rng: number;
  isPaused: boolean;
  isGameOver: boolean;
  lastUpdateMs: number;
  dpr: number;
};

export type SimState = Pick<
  GameState,
  | "score"
  | "bestScore"
  | "lives"
  | "levelIndex"
  | "highestLevel"
  | "bricks"
  | "paddle"
  | "balls"
  | "activePowerUps"
  | "fallingPowerUps"
  | "particles"
  | "shakeMsRemaining"
  | "shakeMagnitude"
  | "rng"
  | "isPaused"
  | "isGameOver"
>;


