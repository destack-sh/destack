export * from "./artifacts";
export * from "./executions";
export * from "./spec";

export type LimitPaginatedResult<T> = {
  count: number;
  next?: string;
  previous?: string;
  results: T[];
};
