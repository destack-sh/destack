import { graphql } from "@/gql";
import { useAuth } from "@/state/auth";
import { useQuery } from "@vue/apollo-composable";
import { DateTime } from "luxon";
import { defineStore } from "pinia";

export type DisplayNotification = {
  id?: string; // if from the server, this is the id of the notification
  localId: string;
  type: string;
  kind: "success" | "warning" | "notice" | "error";
  message: string;
  description?: string;
  actionText?: string;
  action?: () => void;
  shownAt?: DateTime;
  showTimeMs?: number;
};

export const useNotificationsStore = defineStore("notifications", {
  state: () => {
    return {
      pastNotifications: [] as DisplayNotification[],
      shownNotifications: [] as DisplayNotification[],
    };
  },
  actions: {
    show(
      notificationData: Partial<DisplayNotification> & Pick<DisplayNotification, "type" | "kind" | "message">
    ): void {
      const notification: DisplayNotification = {
        localId: notificationData.localId ?? Math.random().toString(36).slice(2, 9),
        showTimeMs: notificationData.showTimeMs ?? 7000,
        shownAt: DateTime.now(),
        ...notificationData,
      };
      notification.shownAt = DateTime.now();
      this.pastNotifications.push(notification);
      this.shownNotifications.push(notification);
      setTimeout(() => this.dismiss(notification.localId), notification.showTimeMs);
    },
    showIf(
      notificationData: Partial<DisplayNotification> & Pick<DisplayNotification, "type" | "kind" | "message">,
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
      this.shownNotifications = this.shownNotifications.filter((n) => n.localId !== notificationId);
    },
    dismissIf(filters: { type?: string }): void {
      this.shownNotifications = this.shownNotifications.filter((n) => n.type !== filters.type);
    },
  },
});

export function useNotifications() {
  const store = useNotificationsStore();

  const auth = useAuth();

  // auto-refresh notifications from DB
  const { result: newNotificationsResult } = useQuery(
    graphql(/* GraphQL */ `
      query newNotifications($after: String, $status: NotificationStatus) {
        me {
          notifications(after: $after, filters: { status: $status }) {
            edges {
              node {
                id
                type
                createdAt
                status
              }
            }
          }
        }
      }
    `)
  );

  return store;
  // I wanted to do this but the methods seemed to no-op:
  // return {
  //   store,
  //   show: store.show,
  //   showIf: store.showIf,
  //   dismiss: store.dismiss,
  //   dismissIf: store.dismissIf,
  // };
}
