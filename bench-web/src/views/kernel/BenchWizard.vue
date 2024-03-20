<script lang="tsx" setup>
import { ViewData, NodeReferenceData, Region, UserStatus, Variant } from "@/proto/wire/";
import { useLoadedGraph } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { createBench, user } from "@/system/user";
import { viewEmits, type FocusAnchor } from "@/views/common";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { toRef, type Ref, ref, watch } from "vue";
import Button from "@/views/controls/Button.vue";
import PlainText from "@/views/content/PlainText.vue";
import { toNodeReference } from "@/proto/wiring";
import { removeView, spaceRegistry } from "@/system/space";
import { getViewComponentChildren, isVueInstanceOf } from "@/views/registry";

const props = defineProps<{ self: NodeReferenceData } & Pick<ViewData, "nodePtr">>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));

const self = toRef(props, "self");
const state: Ref<"create-bench" | "all-set"> = ref("create-bench");
const slug: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.EUROPE_CENTRAL);
const isActive = ref(false);
const lastError = ref<RpcError | null>(null);

// sync user/bench state
watch(
  user,
  () => {
    state.value = "create-bench";
    if (user.value) {
      slug.value = user.value.slug ?? "";
      if (user.value.status == UserStatus.ACTIVATED) {
        state.value = "all-set";
      }
    }
  },
  { immediate: true },
);

async function submit() {
  isActive.value = true;
  try {
    if (!user.value) throw new Error("no active user");
    await createBench({ owner: toNodeReference(user.value), slug: slug.value, region: region.value, isMain: true });
  } catch (e) {
    lastError.value = e as RpcError;
  } finally {
    isActive.value = false;
  }
}

const instance = spaceRegistry.registerCurrent(self);
function focus(anchor: FocusAnchor | NodeReferenceData) {
  const childViews = getViewComponentChildren(instance);
  if (anchor != "bottom") {
    return childViews.find((v) => isVueInstanceOf(v, PlainText));
  } else {
    return childViews.reverse().find((v) => isVueInstanceOf(v, Button));
  }
}

defineExpose({ self, focus });
</script>
<template>
  <div
    class="m-4 min-w-80 max-w-96 rounded-md border border-gray-300 bg-white px-9 py-7 text-gray-900 shadow-md shadow-gray-300"
  >
    <!-- Header -->
    <div>
      <h2 class="text-2xl font-semibold">Create your Bench</h2>
      <p class="mt-2 text-gray-500">
        <span v-if="state == 'create-bench'">You're off the waitlist. Let's go!</span>
        <span v-else-if="state == 'all-set'">You already have a Bench.</span>
      </p>
    </div>
    <!-- Data -->
    <div v-if="state == 'create-bench'" class="mt-5">
      <!-- Owner -->
      <!-- ... -->
      <!-- Slug must match user slug for main bench -->
      <PlainText :icon="makeIcon({ name: 'fas fa-at' })" name="slug" title="Slug" is-input is-disabled v-model="slug" />
      <!-- Region -->
      <!-- ... -->
    </div>
    <!-- Actions -->
    <div class="mt-7">
      <Button
        v-if="state == 'create-bench'"
        name="submit"
        :icon="makeIcon({ name: 'fa-rock' })"
        title="Create Bench"
        class="w-full"
        :is-loading="isActive"
        :is-disabled="isActive"
        @click="submit"
      />
      <Button
        v-if="state == 'all-set'"
        name="close"
        :icon="makeIcon({ name: 'fas fa-xmark' })"
        title="Close"
        class="w-full"
        :variant="Variant.V3"
        @click="() => removeView(spaceConnection.sideTx, spaceGraph, spaceGraph.get(self) as ViewData)"
      />
      <!-- Error -->
      <p v-if="lastError" class="mt-4 text-sm font-semibold text-danger-500">
        {{ lastError.code }}: {{ lastError.message }}
      </p>
    </div>
  </div>
</template>
