let currentSeriesPerMs: Record<number, number> = {};

function getSeries(unixTimeMs: number): number {
  const series = currentSeriesPerMs[unixTimeMs] ?? 0;
  if (Object.keys(currentSeriesPerMs).length > 10_000) {
    currentSeriesPerMs = {};
    currentSeriesPerMs[unixTimeMs] = series;
  }
  currentSeriesPerMs[unixTimeMs] = series + 1;
  currentSeriesPerMs[unixTimeMs] %= 65_536;
  return series;
}

function randomBytes(length: number): string {
  return Array.from({ length: length * 2 }, () => Math.floor(Math.random() * 16).toString(16)).join("");
}

/**
 * Generates a UUIDT (like UUID but timebased) string (formatted as a UUID, see bench backend UUIDT).
 * If provided the nonce must be 8 bytes long (16 hex character string).
 *
 * The format is:
 * - 6 bytes - Unix time milliseconds unsigned integer
 * - 2 bytes - auto-incremented series unsigned integer
 *  (per millisecond, rolls over to 0 after reaching 65 535 UUIDs in one ms)
 * - 8 bytes - securely random gibberish
 *  */
export function uuidt(options?: { nonce?: string }): string {
  const unixTimeMs = Date.now();
  const timePart = unixTimeMs.toString(16).padStart(12, "0");
  const seriesPart = getSeries(unixTimeMs).toString(16).padStart(4, "0");
  const randomPart = options?.nonce ?? randomBytes(8);
  return `${timePart.slice(0, 8)}-${timePart.slice(8, 12)}-${seriesPart.slice(0, 4)}-${randomPart.slice(0, 4)}-${randomPart.slice(4, 12)}`;
}
