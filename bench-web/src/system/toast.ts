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
};

export enum ToastDuration {
  sm = 5000,
  md = 7000,
  lg = 12000,
  "2xl" = 20000,
  "3xl" = 30000,
  inf = Infinity,
}

export type ToastIn = Pick<Toast, "title" | "text" | "level"> & { icon?: string | IconData } & Partial<
    Pick<Toast, "actions" | "durationMs">
  >;

export class Toaster {
  toasts: Ref<Toast[]>;

  constructor() {
    this.toasts = ref([]);
  }

  get activeToasts(): Toast[] {
    return this.toasts.value.filter((t) => t.remainingDurationMs > 0);
  }

  add(toast: ToastIn) {
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

  /** Updates all toasts at the current time, pruning any dead ones. */
  private tick() {
    const now = DateTime.now();
    for (const toast of this.toasts.value) {
      if (toast.remainingDurationMs != 0) {
        toast.remainingDurationMs = Math.max(0, toast.durationMs - now.diff(toast.createdAt).milliseconds);
      }
    }
    this.toasts.value = this.toasts.value.filter((t) => t.remainingDurationMs > 0);
  }

  run() {
    setInterval(() => this.tick(), 100);
  }
}

export const toaster = new Toaster();
