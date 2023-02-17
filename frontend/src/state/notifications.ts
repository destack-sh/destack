import { DateTime } from "luxon";
import { defineStore } from "pinia";

export type Notification = {
  id: string;
  type: string;
  kind: "success" | "warning" | "notice" | "error";
  message: string;
  description?: string;
  actionText?: string;
  action?: () => void;
  shownAt?: DateTime;
  showTimeMs?: number;
  source: "editor";
};

export const useNotifications = defineStore("notifications", {
  state: () => {
    return {
      pastNotifications: [] as Notification[],
      activeNotifications: [] as Notification[],
    };
  },
  actions: {
    show(notificationData: Partial<Notification> & Pick<Notification, "type" | "kind" | "message">): void {
      const notification: Notification = {
        id: notificationData.id ?? Math.random().toString(36).slice(2, 9),
        showTimeMs: notificationData.showTimeMs ?? 7000,
        shownAt: DateTime.now(),
        source: notificationData.source ?? "editor",
        ...notificationData,
      };
      notification.shownAt = DateTime.now();
      this.pastNotifications.push(notification);
      this.activeNotifications.push(notification);
      setTimeout(() => this.dismiss(notification.id), notification.showTimeMs);
    },
    showIf(
      notificationData: Partial<Notification> & Pick<Notification, "type" | "kind" | "message">,
      condition: { lastActiveMs?: number }
    ): void {
      if (condition.lastActiveMs != null) {
        const now = DateTime.now();
        const latestNotificationOfType = this.pastNotifications.find(
          (n) =>
            n.type == notificationData.type &&
            now.diff(n.shownAt as DateTime).as("milliseconds") < (condition.lastActiveMs as number)
        );
        if (latestNotificationOfType != null) {
          return;
        }
      }
      this.show(notificationData);
    },
    dismiss(notificationId: string): void {
      this.activeNotifications = this.activeNotifications.filter((n) => n.id !== notificationId);
    },
    dismissIf(filters: { type?: string }): void {
      this.activeNotifications = this.activeNotifications.filter((n) => n.type !== filters.type);
    },
  },
});
