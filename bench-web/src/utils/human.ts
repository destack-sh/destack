/**
 * Formats numbers into their highest 3-exponent of 10 (k, m, b)
 *  (like 57 -> 57, 7207 -> 7.2k, 2000000 -> 2m)
 */
export function humanizeNumber(num: number): string {
  if (num < 1000) {
    return num.toString();
  } else if (num < 1000000) {
    return `${Math.round(num / 100) / 10}k`;
  } else if (num < 1000000000) {
    return `${Math.round(num / 100000) / 10}m`;
  } else {
    return `${Math.round(num / 100000000) / 10}b`;
  }
}

/** Formats a number into bytes. */
export function humanizeBytes(bytes: number, options?: { cutoff?: number; round?: boolean }) {
  const cutoff = options?.cutoff ?? 100;
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let unit = 0;
  while (bytes >= cutoff && unit < units.length - 1) {
    bytes /= 1024;
    unit++;
  }
  if (options?.round) {
    return `${Math.round(bytes)}${units[unit]}`;
  } else if (unit == 0) {
    return `${bytes.toFixed(0)}${units[unit]}`;
  } else if (bytes < 10) {
    return `${bytes.toFixed(1)}${units[unit]}`;
  } else {
    return `${bytes.toFixed(0)}${units[unit]}`;
  }
}
