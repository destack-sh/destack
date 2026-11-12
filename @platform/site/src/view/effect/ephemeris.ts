/** A body the sky draws where it stands. */
export type Body = "sun" | "moon" | "mercury" | "venus" | "mars" | "jupiter" | "saturn";

/** A planet the orbits list, or the Earth and Moon's barycentre they are seen from. */
export type Orbiter = Exclude<Body, "sun" | "moon"> | "earth";

/**
 * An orbit's Keplerian elements at J2000 and their change a century.
 *
 * The six elements are the semi-major axis in AU, the eccentricity, and in degrees the inclination,
 * the mean longitude, the longitude of perihelion and the longitude of the ascending node.
 */
type Orbit = { elements: readonly number[]; rates: readonly number[] };

/** The clock the sky reads the time from and repeats on. */
export type SkyClock = {
    now: () => number;
    setInterval: (callback: () => void, interval: number) => unknown;
};

/** The orbits from JPL's approximate positions of the major planets, valid 1800 to 2050 AD. */
export const ORBITS: Readonly<Record<Orbiter, Orbit>> = {
    mercury: {
        elements: [0.38709927, 0.20563593, 7.00497902, 252.2503235, 77.45779628, 48.33076593],
        rates: [0.00000037, 0.00001906, -0.00594749, 149472.67411175, 0.16047689, -0.12534081],
    },
    venus: {
        elements: [0.72333566, 0.00677672, 3.39467605, 181.9790995, 131.60246718, 76.67984255],
        rates: [0.0000039, -0.00004107, -0.0007889, 58517.81538729, 0.00268329, -0.27769418],
    },
    earth: {
        elements: [1.00000261, 0.01671123, -0.00001531, 100.46457166, 102.93768193, 0],
        rates: [0.00000562, -0.00004392, -0.01294668, 35999.37244981, 0.32327364, 0],
    },
    mars: {
        elements: [1.52371034, 0.0933941, 1.84969142, -4.55343205, -23.94362959, 49.55953891],
        rates: [0.00001847, 0.00007882, -0.00813131, 19140.30268499, 0.44441088, -0.29257343],
    },
    jupiter: {
        elements: [5.202887, 0.04838624, 1.30439695, 34.39644051, 14.72847983, 100.47390909],
        rates: [-0.00011607, -0.00013253, -0.00183714, 3034.74612775, 0.21252668, 0.20469106],
    },
    saturn: {
        elements: [9.53667594, 0.05386179, 2.48599187, 49.95424423, 92.59887831, 113.66242448],
        rates: [-0.0012506, -0.00050991, 0.00193609, 1222.49362201, -0.41897216, -0.28867794],
    },
};

/**
 * Write the sky over a longitude onto a root's custom properties, and again every interval.
 *
 * The page inlines it in its head, so it reads nothing but its arguments.
 * It writes the local sidereal time in hours as --sky-hours.
 * It writes each body's place as --sky-<body>-x, its hour angle in turns west of the meridian,
 * and --sky-<body>-y, its distance down from the north pole in turns of 180 degrees.
 * It writes the Moon's lit share as --sky-moon-light, positive when its east side is lit.
 */
export function writeSky(
    orbits: Readonly<Record<Orbiter, Orbit>>,
    longitude: number,
    interval: number,
    clock: SkyClock,
    root: HTMLElement,
): void {
    // convert between degrees and radians
    const radians = Math.PI / 180;
    const degrees = 180 / Math.PI;
    const obliquity = 23.4393 * radians;

    // place an orbiter around the Sun at a time in Julian centuries, in AU on the ecliptic
    const place = (orbit: Orbit, centuries: number) => {
        // move the elements to the time
        const [axis = 0, eccentricity = 0, inclination = 0, mean = 0, perihelion = 0, node = 0] =
            orbit.elements.map((element, index) => element + (orbit.rates[index] ?? 0) * centuries);

        // solve Kepler's equation for the eccentric anomaly
        const anomaly = ((((mean - perihelion) % 360) + 540) % 360) - 180;
        let eccentric = anomaly * radians + eccentricity * Math.sin(anomaly * radians);
        for (let step = 0; step < 6; step++) {
            eccentric -=
                (eccentric - eccentricity * Math.sin(eccentric) - anomaly * radians) /
                (1 - eccentricity * Math.cos(eccentric));
        }

        // find the place in the orbit's plane
        const across = axis * (Math.cos(eccentric) - eccentricity);
        const along = axis * Math.sqrt(1 - eccentricity ** 2) * Math.sin(eccentric);

        // turn the orbit's plane onto the ecliptic
        const argument = (perihelion - node) * radians;
        const ascending = node * radians;
        const tilt = inclination * radians;
        const [cosArgument, sinArgument] = [Math.cos(argument), Math.sin(argument)];
        const [cosNode, sinNode] = [Math.cos(ascending), Math.sin(ascending)];
        const [cosTilt, sinTilt] = [Math.cos(tilt), Math.sin(tilt)];

        return {
            x:
                (cosArgument * cosNode - sinArgument * sinNode * cosTilt) * across +
                (-sinArgument * cosNode - cosArgument * sinNode * cosTilt) * along,
            y:
                (cosArgument * sinNode + sinArgument * cosNode * cosTilt) * across +
                (-sinArgument * sinNode + cosArgument * cosNode * cosTilt) * along,
            z: sinArgument * sinTilt * across + cosArgument * sinTilt * along,
        };
    };

    // write the sky at the current time
    const write = () => {
        // count the days and centuries since J2000
        const days = clock.now() / 86_400_000 - 10_957.5;
        const centuries = days / 36_525;

        // turn the sidereal clock to the longitude
        const hours = (((18.697374558 + 24.06570982441908 * days + longitude / 15) % 24) + 24) % 24;
        root.style.setProperty("--sky-hours", hours.toFixed(4));

        // see each body from the Earth, the Sun opposite the Earth, the Moon by its main terms
        const earth = place(orbits.earth, centuries);
        const lunar =
            218.316 + 13.176396 * days + 6.289 * Math.sin((134.963 + 13.064993 * days) * radians);
        const latitude = 5.128 * Math.sin((93.272 + 13.22935 * days) * radians);
        const sun = { x: -earth.x, y: -earth.y, z: -earth.z };
        const moon = {
            x: Math.cos(latitude * radians) * Math.cos(lunar * radians),
            y: Math.cos(latitude * radians) * Math.sin(lunar * radians),
            z: Math.sin(latitude * radians),
        };
        const seen: Record<string, { x: number; y: number; z: number }> = { sun, moon };
        for (const name of ["mercury", "venus", "mars", "jupiter", "saturn"] as const) {
            const planet = place(orbits[name], centuries);
            seen[name] = { x: planet.x - earth.x, y: planet.y - earth.y, z: planet.z - earth.z };
        }

        // turn each body onto the equator, then write its hour angle and polar distance
        for (const [name, point] of Object.entries(seen)) {
            const north = point.y * Math.sin(obliquity) + point.z * Math.cos(obliquity);
            const east = point.y * Math.cos(obliquity) - point.z * Math.sin(obliquity);
            const ascension = Math.atan2(east, point.x) * degrees;
            const declination = Math.atan2(north, Math.hypot(point.x, east)) * degrees;
            const angle = ((((hours * 15 - ascension) % 360) + 540) % 360) - 180;
            root.style.setProperty(`--sky-${name}-x`, (angle / 360).toFixed(5));
            root.style.setProperty(`--sky-${name}-y`, ((90 - declination) / 180).toFixed(5));
        }

        // light the Moon's side toward the Sun, by the share of its face lit
        const facing =
            (sun.x * moon.x + sun.y * moon.y + sun.z * moon.z) / Math.hypot(sun.x, sun.y, sun.z);
        const lit = (1 - facing / Math.hypot(moon.x, moon.y, moon.z)) / 2;
        const ahead = Math.atan2(sun.y, sun.x) - Math.atan2(moon.y, moon.x);
        root.style.setProperty("--sky-moon-light", (Math.sin(ahead) > 0 ? lit : -lit).toFixed(3));
    };

    // write now, before the first paint, then once an interval
    write();
    clock.setInterval(write, interval);
}
