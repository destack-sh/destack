export type LimitPaginatedResult<T> = {
  count: number;
  next?: string;
  previous?: string;
  results: T[];
};
