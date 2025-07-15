/** Get a value from the environment (optional) */
export function getFromEnvMaybe<T>(key: string, typ: "string"): string | undefined;
export function getFromEnvMaybe<T>(key: string, typ: "number"): number | undefined;
export function getFromEnvMaybe<T>(key: string, typ: "boolean"): boolean | undefined;
export function getFromEnvMaybe<T>(
  key: string,
  typ: "string" | "number" | "boolean",
): T | undefined {
  const value =
    typeof process !== "undefined" ? process.env[key] : (import.meta as any).env["VITE_" + key];
  if (value === undefined) {
    return undefined;
  }

  switch (typ) {
    case "string":
      return value as T;
    case "number":
      const num = Number(value);
      return (isNaN(num) ? undefined : num) as T;
    case "boolean":
      return (value.toLowerCase() === "true" || value === "1") as T;
    default:
      return value as T;
  }
}

/** Get a value from the environment (required) */
export function getFromEnv<T>(key: string, typ: "string"): string;
export function getFromEnv<T>(key: string, typ: "number"): number;
export function getFromEnv<T>(key: string, typ: "boolean"): boolean;
export function getFromEnv<T>(key: string, typ: "string" | "number" | "boolean"): T {
  const value = getFromEnvMaybe(key, typ as any);
  if (value === undefined) {
    throw new Error(`Environment variable ${key} is not set`);
  }
  return value as T;
}

export const IS_PROD = false;
export const IS_DEV = true;
export const IS_TEST = false;
export const IS_WEB = typeof window !== "undefined";
