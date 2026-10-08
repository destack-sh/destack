/** The most ripples the waterline carries at once. */
const rippleCapacity = 8;
/** How fast ripples run outward along the waterline, in CSS pixels per second. */
const rippleSpeed = 90;
/** The seconds a ripple takes to die away. */
const rippleLife = 3;

/** Recent stirs of the waterline: where across the water canvas, when in seconds, and how hard. */
const ripples: { x: number; at: number; strength: number }[] = [];

/** Stir the waterline at a position across the water canvas, sending ripples outward. */
export function stir(x: number, strength: number) {
    ripples.push({ x, at: performance.now() / 1000, strength });
    if (ripples.length > rippleCapacity) {
        ripples.shift();
    }
}

/** Return how far a wavy waterline rises or falls at a position across the canvas, in CSS pixels. */
export function waveAt(x: number, seconds: number) {
    // sum three rolling waves
    const long = Math.sin(x * 0.017 + seconds * 0.7) * 1.8;
    const middle = Math.sin(x * 0.043 - seconds * 1.0) * 0.8;
    const short = Math.sin(x * 0.11 + seconds * 1.4) * 0.3;

    // add the ripples running along the waterline, exactly as the shader does
    let lift = 0;
    for (const ripple of ripples) {
        const age = seconds - ripple.at;
        if (age < 0 || age > rippleLife) {
            continue;
        }
        const gap = Math.abs(x - ripple.x) - age * rippleSpeed;
        lift +=
            ripple.strength *
            Math.exp(-age * 1.4) *
            Math.exp(-(gap * gap) / 484) *
            Math.sin(gap * 0.25);
    }

    return long + middle + short + lift;
}
