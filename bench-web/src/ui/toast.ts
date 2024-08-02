import { LogLevel, type IconData, type TextData } from "@/proto/wire";
import { makeIcon } from "@/ui/icon";
import { DateTime } from "luxon";
import { ref, type Ref } from "vue";

export type ToastAnchor = "top-left" | "top-right" | "bottom-left" | "bottom-right";

/**
 * A mini-Action, displayed inline in a Toast.
 */
export type ToastAction = {
  icon?: IconData;
  title: string;
  action: () => void;
};

export type ToastSummaryInfo<T> = {
  key: string;
  info: T[];
  title: (infos: T[]) => string;
  text: (infos: T[]) => string;
}

// how long toasts remain alive for animations after they expire
const ZOMBIE_TOAST_DURATION = 1000; // ms

/**
 * A timed 'notification' to the user.
 */
export type Toast = {
  id: string;
  icon?: IconData;
  title: string;
  text?: string | TextData;
  level: LogLevel;
  durationMs: number;
  remainingDurationMs: number;
  createdAt: DateTime;
  actions: ToastAction[];
  override?: string;
  summarize?: ToastSummaryInfo<any>;
};

export enum ToastDuration {
  sm = 6000,
  md = 10000,
  lg = 15000,
  "2xl" = 30000,
  inf = Infinity,
}

export const DEFAULT_TOAST_DURATION_BY_LEVEL: Record<LogLevel, ToastDuration> = {
  [LogLevel.UNSPECIFIED]: ToastDuration.md,
  [LogLevel.TRACE]: ToastDuration.sm,
  [LogLevel.DEBUG]: ToastDuration.sm,
  [LogLevel.INFO]: ToastDuration.md,
  [LogLevel.WARNING]: ToastDuration.md,
  [LogLevel.ERROR]: ToastDuration.lg,
  [LogLevel.CRITICAL]: ToastDuration["2xl"],
};

export type ToastIn<T> = Pick<Toast, "title" | "text" | "level"> &
  Partial<Pick<Toast, "actions" | "durationMs">> & {
    icon?: string | IconData;
    debounce?: boolean;
    override?: string;
    summarize?: ToastSummaryInfo<T>;
  };

/** The official container of Toasts */
export class Toaster {
  toasts: Ref<Toast[]>;

  constructor() {
    this.toasts = ref([]);
  }

  get activeToasts(): Toast[] {
    return this.toasts.value.filter((t) => t.remainingDurationMs > 0);
  }

  add(toast: ToastIn<any>) {
    if (toast.override) {
      // remove existing with same override key
      this.toasts.value = this.toasts.value.filter((t) => t.override != toast.override);
    }
    if (toast.summarize) {
      // augment existing with same summarize key
      const existing = this.toasts.value.find((t) => t.summarize?.key == toast.summarize?.key);
      if (existing != null) {
        existing.summarize!.info.push(...toast.summarize.info);
        existing.title = toast.summarize.title(existing.summarize!.info);
        existing.text = toast.summarize.text(existing.summarize!.info);
        return;
      }
    }
    const id = Math.random().toString(36).substring(2);
    const createdAt = DateTime.now();
    const durationMs = toast.durationMs ?? DEFAULT_TOAST_DURATION_BY_LEVEL[toast.level];
    const remainingDurationMs = durationMs;
    const actions = toast.actions ?? [];
    const icon = typeof toast.icon == "string" ? makeIcon({ faName: toast.icon }) : toast.icon;
    this.toasts.value.push({ ...toast, icon, durationMs, actions, id, createdAt, remainingDurationMs });
  }

  trace(toast: Omit<ToastIn<any>, "level">) {
    this.add({ ...toast, level: LogLevel.TRACE });
  }

  debug(toast: Omit<ToastIn<any>, "level">) {
    this.add({ ...toast, level: LogLevel.DEBUG });
  }

  info(toast: Omit<ToastIn<any>, "level">) {
    this.add({ ...toast, level: LogLevel.INFO });
  }

  warning(toast: Omit<ToastIn<any>, "level">) {
    this.add({ ...toast, level: LogLevel.WARNING });
  }

  error(toast: Omit<ToastIn<any>, "level">) {
    this.add({ ...toast, level: LogLevel.ERROR });
  }

  critical(toast: Omit<ToastIn<any>, "level">) {
    this.add({ ...toast, level: LogLevel.CRITICAL });
  }

  dismiss(toast: Toast) {
    toast.remainingDurationMs = 0;
  }

  /** Updates all toasts at the current time, pruning as needed. */
  private tick() {
    const now = DateTime.now();
    for (const toast of this.toasts.value) {
      if (toast.remainingDurationMs > 0) {
        toast.remainingDurationMs = toast.durationMs - now.diff(toast.createdAt).milliseconds;
      }
    }
    this.toasts.value = this.toasts.value.filter((t) => t.remainingDurationMs > -ZOMBIE_TOAST_DURATION);
  }

  run() {
    setInterval(() => this.tick(), 250);
  }
}

export const toaster = new Toaster();
