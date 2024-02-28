import type { BadgeData, ClientData } from "@/proto/wire";
import { ref } from "vue";

type BadgeInfo = Pick<BadgeData, "id" | "key" | "password">;
type ClientInfo = Pick<ClientData, "id" | "deviceName" | "browserName"> & { nonce: string };

const authState = {
  client: ref<ClientInfo | null>(null),
  clientToken: ref<string | null>(null),
  badgesById: ref<{ [id: string]: BadgeInfo }>({}),

  get isAuthenticated() {
    return this.clientToken.value !== null;
  },

  get badges() {
    return Object.values(this.badgesById.value);
  },
};

export default authState;
