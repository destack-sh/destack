import {
    BALL_MAX_SPEED_PX_PER_S,
    BALL_SPEED_INCREMENT_ON_HIT,
    VIRTUAL_HEIGHT,
    VIRTUAL_WIDTH,
} from "./constants";
import type { Ball, Brick, Paddle, Rect } from "./types";

export function clamp(value: number, min: number, max: number): number {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

export function rectContainsPoint(rect: Rect, px: number, py: number): boolean {
    return px >= rect.x && px <= rect.x + rect.w && py >= rect.y && py <= rect.y + rect.h;
}

export function ballIntersectsBrick(ball: Ball, brick: Brick): boolean {
    if (!brick.alive) return false;
    const closestX = clamp(ball.x, brick.x, brick.x + brick.w);
    const closestY = clamp(ball.y, brick.y, brick.y + brick.h);
    const dx = ball.x - closestX;
    const dy = ball.y - closestY;
    return dx * dx + dy * dy <= ball.r * ball.r;
}

export function handleBallPaddleCollision(ball: Ball, paddle: Paddle): void {
    if (!aabbCircleOverlap(paddle, ball)) return;
    // reflect vertically and add english based on hit offset
    const offset = (ball.x - (paddle.x + paddle.w / 2)) / (paddle.w / 2);
    const angle = offset * (Math.PI / 3); // up to 60deg from vertical
    const speed = Math.min(ball.speed + BALL_SPEED_INCREMENT_ON_HIT, BALL_MAX_SPEED_PX_PER_S);
    ball.vx = Math.sin(angle) * speed;
    ball.vy = -Math.cos(angle) * speed;
    ball.speed = speed;
}

export function aabbCircleOverlap(rect: Rect, circle: Ball): boolean {
    const closestX = clamp(circle.x, rect.x, rect.x + rect.w);
    const closestY = clamp(circle.y, rect.y, rect.y + rect.h);
    const dx = circle.x - closestX;
    const dy = circle.y - closestY;
    return dx * dx + dy * dy <= circle.r * circle.r;
}

// Decide the reflection axis based on previous position relative to brick.
export function resolveBallBrickCollision(ball: Ball, brick: Brick): "x" | "y" | null {
    if (!ballIntersectsBrick(ball, brick)) return null;
    // decide whether to reflect x or y based on penetration direction
    const prevX = ball.x - ball.vx * (1 / Math.max(ball.speed, 1));
    const prevY = ball.y - ball.vy * (1 / Math.max(ball.speed, 1));
    const wasLeft = prevX < brick.x;
    const wasRight = prevX > brick.x + brick.w;
    const wasAbove = prevY < brick.y;
    const wasBelow = prevY > brick.y + brick.h;

    if ((wasLeft && !wasAbove && !wasBelow) || (wasRight && !wasAbove && !wasBelow)) {
        ball.vx = -ball.vx;
        return "x";
    }
    ball.vy = -ball.vy;
    return "y";
}

export function integrateBall(ball: Ball, dt: number): void {
    ball.x += ball.vx * dt;
    ball.y += ball.vy * dt;

    // walls
    if (ball.x - ball.r < 0) {
        ball.x = ball.r;
        ball.vx = Math.abs(ball.vx);
    } else if (ball.x + ball.r > VIRTUAL_WIDTH) {
        ball.x = VIRTUAL_WIDTH - ball.r;
        ball.vx = -Math.abs(ball.vx);
    }
    if (ball.y - ball.r < 0) {
        ball.y = ball.r;
        ball.vy = Math.abs(ball.vy);
    }
}

export function isBallLost(ball: Ball): boolean {
    return ball.y - ball.r > VIRTUAL_HEIGHT + 40;
}
