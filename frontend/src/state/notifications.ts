import { graphql } from "@/gql";
import { NotificationStatus, NotificationType, type Notification } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { useOperations } from "@/state/operations";
import { UserPlusIcon } from "@heroicons/vue/24/outline";
import { useMutation, useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/shared";
import { DateTime } from "luxon";
import { defineStore } from "pinia";
import { computed, shallowRef, toRef, watch } from "vue";

export type DisplayNotification = {
  id?: string; // if from the server, this is the id of the notification
  notification?: Notification; // if server notification
  localId: string;
  type: string;
  kind: "success" | "warning" | "notice" | "error";
  icon?: any;
  message: string;
  description?: string;
  actionText?: string;
  action?: () => void;
  shownAt?: DateTime;
  showTimeMs?: number;
  initialShowTimeMs?: number;
};

function getTimeShownMs(notification: DisplayNotification): number {
  return DateTime.now()
    .diff(notification.shownAt as DateTime)
    .as("milliseconds");
}

function dismissNotificationWhenExpired(dismiss: () => void, notification: DisplayNotification) {
  setTimeout(() => {
    // check if time has really elapsed (e.g. if freezing while hovering)
    const timeShownMs = getTimeShownMs(notification);
    if (timeShownMs > (notification.showTimeMs as number)) {
      dismiss();
    } else {
      // try again
      dismissNotificationWhenExpired(dismiss, notification);
    }
  }, notification.showTimeMs);
}

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
        localId: notificationData.localId ?? notificationData.id ?? Math.random().toString(36).slice(2, 9),
        showTimeMs: notificationData.showTimeMs ?? 7000,
        shownAt: DateTime.now(),
        icon: notificationData.icon ? shallowRef(notificationData.icon) : undefined,
        ...notificationData,
      };
      notification.initialShowTimeMs = notification.showTimeMs;
      notification.shownAt = DateTime.now();
      this.pastNotifications.push(notification);
      this.shownNotifications.push(notification);
      dismissNotificationWhenExpired(() => this.dismiss(notification.localId), notification);
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
    freeze(notificationId: string): void {
      // reset the notifications show time
      const notification = this.shownNotifications.find((n) => n.localId == notificationId);
      if (notification != null) {
        notification.showTimeMs = getTimeShownMs(notification) + (notification.initialShowTimeMs as number);
      }
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
          id
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
                organizationInvite {
                  id
                  organization {
                    id
                    slug
                    name
                  }
                  level
                }
                projectInvite {
                  id
                  project {
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
      enabled: toRef(auth, "loggedIn") as any,
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

  // watch and show new server-side notifications
  const handler = _useNotificationHandler();
  watch(serverNotifications, () => {
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
      const rendered = handler.render(notification as Notification);
      if (rendered == null) {
        console.warn("received notification that could not be rendered: ", notification);
        continue;
      }
      store.show({
        id: notification.id,
        showTimeMs: 15000,
        ...rendered,
      });
    }
  });

  return {
    store,
    pastNotifications: computed(() => store.pastNotifications),
    shownNotifications: computed(() => store.shownNotifications),
    activeCount,
    mark: handler.mark,
    render: handler.render,
    refetchFromServer,
    show: store.show,
    showIf: store.showIf,
    dismiss: store.dismiss,
    dismissIf: store.dismissIf,
  };
}

function _useNotificationHandler() {
  const notifications = useNotificationsStore();
  const ops = useOperations();

  const { mutate: markNotificationMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation markNotification($id: GlobalID!, $status: NotificationStatus!) {
        markNotification(input: { id: $id, status: $status }) {
          ... on Notification {
            id
            status
            readAt
            archivedAt
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; status: NotificationStatus }) =>
        ({
          __typename: "Mutation",
          markNotification: {
            __typename: "Notification",
            id: vars.id,
            status: vars.status,
            readAt: vars.status == NotificationStatus.Read ? DateTime.now().toISO() : null,
            archivedAt: vars.status == NotificationStatus.Archived ? DateTime.now().toISO() : null,
          },
        } as any),
      refetchQueries: ["newNotifications"],
    }
  );

  async function mark(notificationId: string, status: NotificationStatus) {
    await ops.state.perform({
      type: "user.markNotification",
      stateless: true,
      do: async () => {
        await markNotificationMut({ id: notificationId, status: status });
      },
    });
  }

  function render(
    notification: Notification
  ):
    | Pick<DisplayNotification, "type" | "kind" | "message" | "description" | "icon" | "actionText" | "action">
    | undefined {
    if (notification.type == NotificationType.OrganizationInvite) {
      return {
        type: "user.receivedOrganizationInvite",
        kind: "notice",
        message: `${notification.organizationInvite.organization.slug} invited you`,
        description: `Join them to work on AI together.`,
        icon: UserPlusIcon,
        actionText: "Accept",
        action: async () => {
          const ret = await ops.user.acceptOrganizationInvite(notification.organizationInvite.id);
          if (ret?.data?.acceptOrganizationInvite?.__typename == "User") {
            // success
            notifications.show({
              type: "user.acceptedOrganizationInvite",
              kind: "success",
              message: `Joined ${notification.organizationInvite.organization.slug}`,
              description: `You are now a part of ${notification.organizationInvite.organization.name}!`,
            });
          }
        },
      };
    }
    return undefined;
  }

  return {
    mark,
    render,
  };
}

export const useNotifications = createSharedComposable(_useNotifications);
