import { graphql } from "@/gql";
import { NotificationStatus, type Notification } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import { DateTime } from "luxon";
import { defineStore } from "pinia";
import { computed, toRef, watchEffect } from "vue";

export type DisplayNotification = {
  id?: string; // if from the server, this is the id of the notification
  notification?: Notification;
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

function _useNotifications() {
  const store = useNotificationsStore();

  const auth = useAuth();

  // get notifications from DB
  const { result: newNotificationsResult, refetch: refetchFromServer } = useQuery(
    graphql(/* GraphQL */ `
      query newNotifications($after: String, $status: NotificationStatus) {
        me {
          notifications(after: $after, filters: { status: $status }) {
            totalCount
            edges {
              node {
                id
                type
                createdAt
                readAt
                archivedAt
                expiresAt
                status
                invite {
                  id
                  organization {
                    id
                    slug
                    name
                  }
                  level
                }
              }
            }
          }
        }
      }
    `),
    {
      status: NotificationStatus.Active,
    } as any,
    {
      enabled: toRef(auth, "loggedIn"),
    }
  );

  // auto-refetch every minute (should be a subscription later)
  setInterval(() => {
    if (auth.loggedIn) {
      refetchFromServer();
    }
  }, 60 * 1000);

  const activeCount = computed(() => newNotificationsResult.value?.me?.notifications?.totalCount ?? 0);
  const serverNotifications = computed(
    () => newNotificationsResult.value?.me?.notifications?.edges.map((e) => e.node) ?? []
  );

  // watch for new server-side notifications
  watchEffect(() => {
    if (serverNotifications.value == null) {
      return;
    }
    for (const notification of serverNotifications.value) {
      if (
        store.shownNotifications.find((n) => n.id == notification.id) ||
        store.pastNotifications.find((n) => n.id == notification.id)
      ) {
        // already shown
        continue;
      }
      store.show({
        id: notification.id,
        showTimeMs: 15000,
        ...renderNotification(notification as Notification),
      });
    }
  });

  return {
    store,
    pastNotifications: store.pastNotifications,
    shownNotifications: store.shownNotifications,
    activeCount,
    refetchFromServer,
    show: store.show,
    showIf: store.showIf,
    dismiss: store.dismiss,
    dismissIf: store.dismissIf,
  };
}

export const useNotifications = createSharedComposable(_useNotifications);

export function renderNotification(
  notification: Notification
): Pick<DisplayNotification, "type" | "kind" | "message" | "description"> {
  return {
    type: notification.type,
    kind: "notice",
    message: notification.type,
  };
}
