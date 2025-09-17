import { useCallback, useEffect, useRef, useState } from "react";
import {
    BALL_RADIUS,
    BALL_START_SPEED_PX_PER_S,
    COLORS,
    FRAME_DT_CAP_S,
    FIXED_DT_S,
    MAX_STEPS_PER_FRAME,
    TIME_SCALE,
    PADDLE_BASE_WIDTH,
    PADDLE_HEIGHT,
    PADDLE_SPEED_PX_PER_S,
    START_LIVES,
    VIRTUAL_HEIGHT,
    VIRTUAL_WIDTH,
} from "./constants";
import { audioManager } from "./audio";
import { colorForBrick, createLevelBricks } from "./levels";
import {
    handleBallPaddleCollision,
    integrateBall,
    isBallLost,
    resolveBallBrickCollision,
} from "./physics";
import type { Ball, GameState, InputState, Paddle } from "./types";
import type { SimState } from "./types";
import { rngNext, rngSeedFromTime } from "./rng";

type Props = {
    className?: string;
};

function saveProgress(bestScore: number, highestLevel: number): void {
    localStorage.setItem("breakout_progress_v1", JSON.stringify({ bestScore, highestLevel }));
}

export function Breakout({ className }: Props) {
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const historyRef = useRef<SimState[]>([]);
    const historyMax = 120 * 30 * 60; // 30 minutes at 120Hz
    const [state, setState] = useState<GameState>(() => {
        const saved = localStorage.getItem("breakout_progress_v1");
        const parsed = saved ? (JSON.parse(saved) as Partial<GameState>) : null;
        const paddle: Paddle = {
            x: (VIRTUAL_WIDTH - PADDLE_BASE_WIDTH) / 2,
            y: VIRTUAL_HEIGHT - 60,
            w: PADDLE_BASE_WIDTH,
            h: PADDLE_HEIGHT,
            vx: 0,
            stickyUntilMs: 0,
        };
        const ball: Ball = {
            x: VIRTUAL_WIDTH / 2,
            y: paddle.y - BALL_RADIUS - 1,
            r: BALL_RADIUS,
            vx: 0,
            vy: -BALL_START_SPEED_PX_PER_S,
            speed: BALL_START_SPEED_PX_PER_S,
            stuckToPaddle: true,
        };
        return {
            score: 0,
            bestScore: parsed?.bestScore ?? 0,
            lives: START_LIVES,
            levelIndex: 0,
            highestLevel: parsed?.highestLevel ?? 0,
            bricks: createLevelBricks(0),
            paddle,
            balls: [ball],
            activePowerUps: [],
            fallingPowerUps: [],
            particles: [],
            shakeMsRemaining: 0,
            shakeMagnitude: 0,
            rng: rngSeedFromTime(),
            isPaused: false,
            isGameOver: false,
            lastUpdateMs: performance.now(),
            dpr: Math.max(1, Math.min(3, window.devicePixelRatio || 1)),
        };
    });

    const inputRef = useRef<InputState>({
        left: false,
        right: false,
        launch: false,
        pauseToggle: false,
        rewind: false,
    });

    useEffect(() => {
        const handleKey = (e: KeyboardEvent, down: boolean) => {
            if (e.key === "ArrowLeft" || e.key === "a") inputRef.current.left = down;
            if (e.key === "ArrowRight" || e.key === "d") inputRef.current.right = down;
            if (e.key === " " || e.key === "ArrowUp" || e.key === "w")
                inputRef.current.launch = down;
            if (e.key === "Escape" || e.key === "p") {
                if (down) setState((s) => ({ ...s, isPaused: !s.isPaused }));
            }
            if (e.key === "Shift") inputRef.current.rewind = down;
        };
        const kd = (e: KeyboardEvent) => handleKey(e, true);
        const ku = (e: KeyboardEvent) => handleKey(e, false);
        window.addEventListener("keydown", kd);
        window.addEventListener("keyup", ku);
        return () => {
            window.removeEventListener("keydown", kd);
            window.removeEventListener("keyup", ku);
        };
    }, []);

    // auto-pause when tab hidden
    useEffect(() => {
        const onVis = () => setState((s) => ({ ...s, isPaused: document.hidden || s.isPaused }));
        document.addEventListener("visibilitychange", onVis);
        return () => document.removeEventListener("visibilitychange", onVis);
    }, []);

    // resize for DPR
    useEffect(() => {
        const el = canvasRef.current;
        if (!el) return;
        const ctx = el.getContext("2d");
        if (!ctx) return;
        const handleResize = () => {
            const dpr = Math.max(1, Math.min(3, window.devicePixelRatio || 1));
            el.width = Math.floor(VIRTUAL_WIDTH * dpr);
            el.height = Math.floor(VIRTUAL_HEIGHT * dpr);
            el.style.width = `${VIRTUAL_WIDTH}px`;
            el.style.height = `${VIRTUAL_HEIGHT}px`;
            ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
            setState((s) => ({ ...s, dpr }));
        };
        handleResize();
        const ro = new ResizeObserver(handleResize);
        ro.observe(el);
        return () => ro.disconnect();
    }, []);

    // init audio on first interaction
    useEffect(() => {
        const onPointerDown = async () => {
            await audioManager.init();
            window.removeEventListener("pointerdown", onPointerDown);
        };
        window.addEventListener("pointerdown", onPointerDown);
        return () => window.removeEventListener("pointerdown", onPointerDown);
    }, []);

    /**
     * Advance simulation with fixed timestep and optional rewind.
     * Records compact history snapshots for deterministic reverse.
     */
    const step = useCallback((now: number) => {
        setState((prev) => {
            if (prev.isGameOver) return { ...prev, lastUpdateMs: now };

            // rewind
            if (inputRef.current.rewind) {
                const snap = historyRef.current.pop();
                if (snap) {
                    return {
                        ...prev,
                        ...snap,
                        lastUpdateMs: now,
                    } as GameState;
                }
                return { ...prev, lastUpdateMs: now };
            }

            if (prev.isPaused) return { ...prev, lastUpdateMs: now };

            let accumulator = ((now - prev.lastUpdateMs) / 1000) * TIME_SCALE;
            if (accumulator <= 0) return { ...prev, lastUpdateMs: now };
            if (accumulator > FRAME_DT_CAP_S) accumulator = FRAME_DT_CAP_S;

            const input = inputRef.current;
            const paddle = { ...prev.paddle };
            const balls = prev.balls.map((b) => ({ ...b }));
            const bricks = prev.bricks.map((b) => ({ ...b }));
            const falling = prev.fallingPowerUps.map((p) => ({ ...p }));
            let activePowerUps = prev.activePowerUps.filter((p) => p.activeUntilMs > now);
            let particles = (prev.particles ?? []).map((p) => ({ ...p }));
            let shakeMsRemaining = prev.shakeMsRemaining ?? 0;
            let shakeMagnitude = prev.shakeMagnitude ?? 0;
            let rng = prev.rng ?? 1;

            let score = prev.score;
            let lives = prev.lives;

            let steps = 0;
            while (accumulator >= FIXED_DT_S && steps < MAX_STEPS_PER_FRAME) {
                steps++;
                const dt = FIXED_DT_S;

                // save snapshot for rewind
                const snap = {
                    score,
                    bestScore: prev.bestScore,
                    lives,
                    levelIndex: prev.levelIndex,
                    highestLevel: prev.highestLevel,
                    bricks: bricks.map((b) => ({ ...b })),
                    paddle: { ...paddle },
                    balls: balls.map((b) => ({ ...b })),
                    activePowerUps: activePowerUps.map((p) => ({ ...p })),
                    fallingPowerUps: falling.map((p) => ({ ...p })),
                    particles: particles.map((p) => ({ ...p })),
                    shakeMsRemaining,
                    shakeMagnitude,
                    rng,
                    isPaused: prev.isPaused,
                    isGameOver: prev.isGameOver,
                } as SimState;
                historyRef.current.push(snap);
                if (historyRef.current.length > historyMax) historyRef.current.shift();

                // paddle movement
                const dir = (input.left ? -1 : 0) + (input.right ? 1 : 0);
                paddle.vx = dir * PADDLE_SPEED_PX_PER_S;
                paddle.x += paddle.vx * dt;
                paddle.x = Math.max(0, Math.min(paddle.x, VIRTUAL_WIDTH - paddle.w));

                // launch if stuck
                if (input.launch) {
                    for (const ball of balls) {
                        if (ball.stuckToPaddle) {
                            ball.stuckToPaddle = false;
                            ball.vx = 0;
                            ball.vy = -ball.speed;
                            audioManager.play("start");
                        }
                    }
                }

                // integrate balls and collisions
                for (const ball of balls) {
                    if (ball.stuckToPaddle) {
                        ball.x = paddle.x + paddle.w / 2;
                        ball.y = paddle.y - ball.r - 1;
                        continue;
                    }
                    integrateBall(ball, dt);
                    handleBallPaddleCollision(ball, paddle);
                    for (const brick of bricks) {
                        if (!brick.alive) continue;
                        if (resolveBallBrickCollision(ball, brick)) {
                            brick.hp -= 1;
                            audioManager.play("brick");
                            if (brick.hp <= 0) {
                                brick.alive = false;
                                score += brick.score;
                                // particles
                                for (let i = 0; i < 10; i++) {
                                    particles.push({
                                        x: ball.x,
                                        y: ball.y,
                                        vx: (Math.random() - 0.5) * 220,
                                        vy: (Math.random() - 0.5) * 220,
                                        lifeMs: 400 + Math.random() * 300,
                                        size: 2 + Math.random() * 2,
                                        color: colorForBrick(brick.colorIndex),
                                    });
                                }
                                shakeMsRemaining = 100;
                                shakeMagnitude = 6;
                                // deterministic powerup drop
                                let rand: number;
                                [rng, rand] = rngNext(rng);
                                if (rand < 0.15) {
                                    let r2: number;
                                    [rng, r2] = rngNext(rng);
                                    const kinds = [
                                        "widen",
                                        "shrink",
                                        "slow",
                                        "fast",
                                        "life",
                                    ] as const;
                                    const idx = Math.floor(r2 * kinds.length);
                                    const kind = kinds[idx] ?? "widen";
                                    falling.push({
                                        kind,
                                        x: brick.x + brick.w / 2,
                                        y: brick.y + brick.h / 2,
                                        vy: 160,
                                    });
                                }
                            }
                        }
                    }
                }

                // update falling powerups
                for (const f of falling) {
                    f.y += f.vy * dt;
                    if (f.y > VIRTUAL_HEIGHT + 20) f.vy = 0;
                    if (
                        f.y >= paddle.y &&
                        Math.abs(f.x - (paddle.x + paddle.w / 2)) <= paddle.w / 2
                    ) {
                        audioManager.play("power");
                        if (f.kind === "widen")
                            paddle.w = Math.min(paddle.w + 30, PADDLE_BASE_WIDTH + 80);
                        if (f.kind === "shrink")
                            paddle.w = Math.max(paddle.w - 30, PADDLE_BASE_WIDTH - 40);
                        if (f.kind === "slow") for (const b of balls) b.speed *= 0.85;
                        if (f.kind === "fast") for (const b of balls) b.speed *= 1.15;
                        if (f.kind === "life") lives = Math.min(lives + 1, 5);
                        activePowerUps.push({ kind: f.kind, activeUntilMs: now + 12000 });
                        f.vy = 0;
                    }
                }
                const remainingFalling = falling.filter((f) => f.vy !== 0);

                // remove lost balls
                const remaining = balls.filter((b) => !isBallLost(b));
                if (remaining.length === 0) {
                    lives -= 1;
                    audioManager.play("lose");
                    if (lives <= 0) {
                        const bestScore = Math.max(prev.bestScore, score);
                        const highestLevel = Math.max(prev.highestLevel, prev.levelIndex);
                        saveProgress(bestScore, highestLevel);
                        return {
                            ...prev,
                            score,
                            bestScore,
                            highestLevel,
                            lives: 0,
                            isGameOver: true,
                            lastUpdateMs: now,
                        };
                    }
                    const ball: Ball = {
                        x: VIRTUAL_WIDTH / 2,
                        y: paddle.y - BALL_RADIUS - 1,
                        r: BALL_RADIUS,
                        vx: 0,
                        vy: -BALL_START_SPEED_PX_PER_S,
                        speed: BALL_START_SPEED_PX_PER_S,
                        stuckToPaddle: true,
                    };
                    // reset balls
                    balls.length = 0;
                    balls.push(ball);
                }

                // level complete
                const anyBricks = bricks.some((b) => b.alive);
                if (!anyBricks) {
                    const levelIndex = prev.levelIndex + 1;
                    audioManager.play("win");
                    const nextBricks = createLevelBricks(levelIndex);
                    const paddleNew: Paddle = { ...paddle, x: (VIRTUAL_WIDTH - paddle.w) / 2 };
                    const ball: Ball = {
                        x: VIRTUAL_WIDTH / 2,
                        y: paddleNew.y - BALL_RADIUS - 1,
                        r: BALL_RADIUS,
                        vx: 0,
                        vy: -BALL_START_SPEED_PX_PER_S,
                        speed: BALL_START_SPEED_PX_PER_S,
                        stuckToPaddle: true,
                    };
                    const bestScore = Math.max(prev.bestScore, score);
                    const highestLevel = Math.max(prev.highestLevel, levelIndex);
                    saveProgress(bestScore, highestLevel);
                    return {
                        ...prev,
                        score,
                        bestScore,
                        levelIndex,
                        highestLevel,
                        bricks: nextBricks,
                        paddle: paddleNew,
                        balls: [ball],
                        activePowerUps: [],
                        fallingPowerUps: [],
                        particles,
                        shakeMsRemaining: 0,
                        shakeMagnitude: 0,
                        rng,
                        lastUpdateMs: now,
                    };
                }

                // update particles
                for (const p of particles) {
                    p.x += p.vx * dt;
                    p.y += p.vy * dt;
                    p.vy += 400 * dt;
                    p.lifeMs -= dt * 1000;
                }
                particles = particles.filter((p) => p.lifeMs > 0);
                if (shakeMsRemaining > 0)
                    shakeMsRemaining = Math.max(0, shakeMsRemaining - dt * 1000);

                accumulator -= FIXED_DT_S;
            }

            return {
                ...prev,
                score,
                lives,
                paddle,
                balls,
                bricks,
                fallingPowerUps: falling.filter((f) => f.vy !== 0),
                activePowerUps,
                particles,
                shakeMsRemaining,
                shakeMagnitude,
                rng,
                lastUpdateMs: now,
            };
        });
    }, []);

    useEffect(() => {
        let raf = 0;
        const loop = (t: number) => {
            step(t);
            raf = requestAnimationFrame(loop);
        };
        raf = requestAnimationFrame(loop);
        return () => cancelAnimationFrame(raf);
    }, [step]);

    const draw = useCallback(() => {
        const el = canvasRef.current;
        if (!el) return;
        const ctx = el.getContext("2d");
        if (!ctx) return;
        // background gradient
        const grad = ctx.createLinearGradient(0, 0, 0, VIRTUAL_HEIGHT);
        grad.addColorStop(0, "#0b0b10");
        grad.addColorStop(1, COLORS.background);
        ctx.fillStyle = grad;
        ctx.fillRect(0, 0, VIRTUAL_WIDTH, VIRTUAL_HEIGHT);

        // bricks
        for (const b of state.bricks) {
            if (!b.alive) continue;
            ctx.fillStyle = colorForBrick(b.colorIndex);
            ctx.fillRect(b.x, b.y, b.w, b.h);
        }

        // paddle
        ctx.fillStyle = COLORS.paddle;
        ctx.fillRect(state.paddle.x, state.paddle.y, state.paddle.w, state.paddle.h);

        // balls
        ctx.fillStyle = COLORS.ball;
        for (const ball of state.balls) {
            ctx.beginPath();
            ctx.arc(ball.x, ball.y, ball.r, 0, Math.PI * 2);
            ctx.fill();
        }

        // particles
        for (const p of state.particles ?? []) {
            ctx.fillStyle = p.color;
            ctx.globalAlpha = Math.max(0, Math.min(1, p.lifeMs / 600));
            ctx.fillRect(p.x, p.y, p.size, p.size);
            ctx.globalAlpha = 1;
        }

        // hud
        ctx.fillStyle = COLORS.foreground;
        ctx.font = "16px ui-sans-serif, system-ui, -apple-system";
        ctx.fillText(`Score: ${state.score}`, 16, 24);
        ctx.fillText(`Best: ${state.bestScore}`, 16, 44);
        ctx.fillText(`Lives: ${state.lives}`, VIRTUAL_WIDTH - 100, 24);
        ctx.fillText(`Level: ${state.levelIndex + 1}`, VIRTUAL_WIDTH / 2 - 40, 24);

        if (state.isGameOver) {
            ctx.fillStyle = COLORS.uiBad;
            ctx.font = "48px ui-sans-serif, system-ui, -apple-system";
            ctx.fillText("Game Over", VIRTUAL_WIDTH / 2 - 120, VIRTUAL_HEIGHT / 2);
            ctx.font = "18px ui-sans-serif, system-ui, -apple-system";
            ctx.fillStyle = COLORS.foreground;
            ctx.fillText(
                "Press Space to restart",
                VIRTUAL_WIDTH / 2 - 110,
                VIRTUAL_HEIGHT / 2 + 28,
            );
        }
    }, [state]);

    useEffect(() => {
        draw();
    }, [draw]);

    // restart on game over space press
    useEffect(() => {
        if (!state.isGameOver) return;
        const onKey = (e: KeyboardEvent) => {
            if (e.key === " ") {
                setState((s) => ({
                    ...s,
                    score: 0,
                    bestScore: s.bestScore,
                    lives: START_LIVES,
                    levelIndex: 0,
                    highestLevel: s.highestLevel,
                    bricks: createLevelBricks(0),
                    balls: [
                        {
                            x: VIRTUAL_WIDTH / 2,
                            y: s.paddle.y - BALL_RADIUS - 1,
                            r: BALL_RADIUS,
                            vx: 0,
                            vy: -BALL_START_SPEED_PX_PER_S,
                            speed: BALL_START_SPEED_PX_PER_S,
                            stuckToPaddle: true,
                        },
                    ],
                    isGameOver: false,
                }));
            }
        };
        window.addEventListener("keydown", onKey);
        return () => window.removeEventListener("keydown", onKey);
    }, [state.isGameOver]);

    // pointer movement for paddle
    useEffect(() => {
        const el = canvasRef.current;
        if (!el) return;
        const onMove = (e: PointerEvent) => {
            const rect = el.getBoundingClientRect();
            const px = e.clientX - rect.left;
            setState((s) => ({
                ...s,
                paddle: {
                    ...s.paddle,
                    x: Math.max(0, Math.min(px - s.paddle.w / 2, VIRTUAL_WIDTH - s.paddle.w)),
                },
            }));
        };
        el.addEventListener("pointermove", onMove);
        return () => el.removeEventListener("pointermove", onMove);
    }, []);

    function saveProgress(bestScore: number, highestLevel: number): void {
        localStorage.setItem("breakout_progress_v1", JSON.stringify({ bestScore, highestLevel }));
    }

    return (
        <div className={className}>
            <canvas
                ref={canvasRef}
                width={VIRTUAL_WIDTH}
                height={VIRTUAL_HEIGHT}
                style={{ borderRadius: 8, boxShadow: "0 8px 24px rgba(0,0,0,0.35)" }}
            />
        </div>
    );
}
