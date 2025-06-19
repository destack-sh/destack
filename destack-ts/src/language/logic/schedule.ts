import { EnumType, Graph, StructFrozen, StructType, Supergraph, Struct, Session, BuiltinObject, QueryConnection, NodeType, Node, NodeReference } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:3051 ==== */
export enum DayOfWeek {
  MONDAY = 1,
  TUESDAY = 2,
  WEDNESDAY = 3,
  THURSDAY = 4,
  FRIDAY = 5,
  SATURDAY = 6,
  SUNDAY = 7,
}
/* ==== DESTACK_GENERATED_END:ENUM:3051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:3052 ==== */
export enum Month {
  JANUARY = 1,
  FEBRUARY = 2,
  MARCH = 3,
  APRIL = 4,
  MAY = 5,
  JUNE = 6,
  JULY = 7,
  AUGUST = 8,
  SEPTEMBER = 9,
  OCTOBER = 10,
  NOVEMBER = 11,
  DECEMBER = 12,
}
/* ==== DESTACK_GENERATED_END:ENUM:3052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:3050 ==== */
export enum ScheduleFrequency {
  YEAR = 1,
  MONTH = 2,
  WEEK = 3,
  DAY = 4,
  HOUR = 5,
  MINUTE = 6,
}
/* ==== DESTACK_GENERATED_END:ENUM:3050 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:3001 ==== */
export class Schedule extends Struct {
  frequency: ScheduleFrequency;
  interval: number;
  start: Temporal.ZonedDateTime | null;
  end: Temporal.ZonedDateTime | null;
  count: number | null;
  weekStart: DayOfWeek | null;
  bySetPos: Array<number>;
  byMonth: Array<Month>;
  byMonthDay: Array<number>;
  byYearDay: Array<number>;
  byEaster: Array<number>;
  byWeekNo: Array<number>;
  byWeekDay: Array<DayOfWeek>;
  byHour: Array<number>;
  byMinute: Array<number>;
  bySecond: Array<number>;

  constructor(
    frequency: ScheduleFrequency,
    interval: number,
    start: Temporal.ZonedDateTime | null,
    end: Temporal.ZonedDateTime | null,
    count: number | null,
    weekStart: DayOfWeek | null,
    bySetPos: Array<number>,
    byMonth: Array<Month>,
    byMonthDay: Array<number>,
    byYearDay: Array<number>,
    byEaster: Array<number>,
    byWeekNo: Array<number>,
    byWeekDay: Array<DayOfWeek>,
    byHour: Array<number>,
    byMinute: Array<number>,
    bySecond: Array<number>,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.frequency = frequency;
    this.interval = interval;
    this.start = start;
    this.end = end;
    this.count = count;
    this.weekStart = weekStart;
    this.bySetPos = bySetPos;
    this.byMonth = byMonth;
    this.byMonthDay = byMonthDay;
    this.byYearDay = byYearDay;
    this.byEaster = byEaster;
    this.byWeekNo = byWeekNo;
    this.byWeekDay = byWeekDay;
    this.byHour = byHour;
    this.byMinute = byMinute;
    this.bySecond = bySecond;
  }


  static create(options: {
    frequency: ScheduleFrequency,
    interval?: number,
    start?: Temporal.ZonedDateTime | null,
    end?: Temporal.ZonedDateTime | null,
    count?: number | null,
    weekStart?: DayOfWeek | null,
    bySetPos?: Array<number>,
    byMonth?: Array<Month>,
    byMonthDay?: Array<number>,
    byYearDay?: Array<number>,
    byEaster?: Array<number>,
    byWeekNo?: Array<number>,
    byWeekDay?: Array<DayOfWeek>,
    byHour?: Array<number>,
    byMinute?: Array<number>,
    bySecond?: Array<number>,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Schedule {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Schedule(
      options.frequency,
      options.interval ?? 1,
      options.start ?? null,
      options.end ?? null,
      options.count ?? null,
      options.weekStart ?? null,
      options.bySetPos ?? [],
      options.byMonth ?? [],
      options.byMonthDay ?? [],
      options.byYearDay ?? [],
      options.byEaster ?? [],
      options.byWeekNo ?? [],
      options.byWeekDay ?? [],
      options.byHour ?? [],
      options.byMinute ?? [],
      options.bySecond ?? [],
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:3001 ==== */