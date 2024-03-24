import { LogLevel, type IconData, type TextData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { DateTime } from "luxon";
import { ref, type Ref } from "vue";

export type ToastAnchor = "top-left" | "top-right" | "bottom-left" | "bottom-right";

/**
 * A mini-Action, displayed inline in a Toast.
 */
export type ToastAction = {
  icon?: IconData;
  title: string;
  isPrimary?: boolean;
  action: () => void;
};

// how long toasts remain alive for animations after they expire
const ZOMBIE_TOAST_DURATION = 1000; // ms

/**
 * A timed 'notification' to the user.
 */
export type Toast = {
  id: string;
  key?: string;
  icon?: IconData;
  title: string;
  text?: string | TextData;
  level: LogLevel;
  durationMs: number;
  remainingDurationMs: number;
  createdAt: DateTime;
  actions: ToastAction[];
};

export enum ToastDuration {
  sm = 5000,
  md = 7000,
  lg = 12000,
  "2xl" = 20000,
  "3xl" = 30000,
  inf = Infinity,
}

export type ToastIn = Pick<Toast, "title" | "text" | "level"> &
  Partial<Pick<Toast, "key" | "actions" | "durationMs">> & { icon?: string | IconData; debounce?: boolean };

/** The official container of Toasts */
export class Toaster {
  toasts: Ref<Toast[]>;

  constructor() {
    this.toasts = ref([]);
  }

  get activeToasts(): Toast[] {
    return this.toasts.value.filter((t) => t.remainingDurationMs > 0);
  }

  hasActiveKey(key: string): boolean {
    return this.activeToasts.some((t) => t.key === key);
  }

  add(toast: ToastIn) {
    if (toast.debounce && this.hasActiveKey(toast.key!)) return;
    const id = Math.random().toString(36).substring(2);
    const createdAt = DateTime.now();
    const durationMs = toast.durationMs ?? ToastDuration.md;
    const remainingDurationMs = durationMs;
    const actions = toast.actions ?? [];
    const icon = typeof toast.icon == "string" ? makeIcon({ name: toast.icon }) : toast.icon;
    this.toasts.value.push({ ...toast, icon, durationMs, actions, id, createdAt, remainingDurationMs });
  }

  trace(toast: Omit<ToastIn, "level">) {
    this.add({ ...toast, level: LogLevel.TRACE });
  }

  debug(toast: Omit<ToastIn, "level">) {
    this.add({ ...toast, level: LogLevel.DEBUG });
  }

  info(toast: Omit<ToastIn, "level">) {
    this.add({ ...toast, level: LogLevel.INFO });
  }

  warning(toast: Omit<ToastIn, "level">) {
    this.add({ ...toast, level: LogLevel.WARNING });
  }

  error(toast: Omit<ToastIn, "level">) {
    this.add({ ...toast, level: LogLevel.ERROR });
  }

  fatal(toast: Omit<ToastIn, "level">) {
    this.add({ ...toast, level: LogLevel.FATAL });
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
