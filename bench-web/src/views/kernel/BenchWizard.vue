<script lang="ts" setup>
import { ViewData, NodeReferenceData, Region, UserStatus, Variant } from "@/proto/wire/";
import { useExistingConnection } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { createBench, user } from "@/system/user";
import { viewEmits, type FocusAnchor } from "@/views/common";
import { toRef, type Ref, ref, watch } from "vue";
import Button from "@/views/controls/Button.vue";
import HtmlInput from "@/views/content/HtmlInput.vue";
import { toNodeReference } from "@/proto/wiring";
import { canvas, goToBench } from "@/system/space";
import { getViewComponentChildren, isVueInstanceOf } from "@/views/canvas";

const props = defineProps<{ self: NodeReferenceData } & Pick<ViewData, "nodePtr">>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(toRef(props, "self"));

type State = "create-bench" | "activate-bench" | "all-set";
const self = toRef(props, "self");
const state: Ref<State> = ref("create-bench");
const slug: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.EUROPE_CENTRAL);
const isActive = ref(false);

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
    const { bench } = await createBench({
      owner: toNodeReference(user.value),
      slug: slug.value,
      region: region.value,
      isMain: true,
    });
    await goToBench({ bench: toNodeReference(bench) });
  } finally {
    isActive.value = false;
  }
}

const instance = canvas.registerView(self);
function focus(anchor: FocusAnchor | NodeReferenceData) {
  const childViews = getViewComponentChildren(instance);
  if (anchor != "bottom") {
    return childViews.find((v) => isVueInstanceOf(v, HtmlInput));
  } else {
    return childViews.reverse().find((v) => isVueInstanceOf(v, Button));
  }
}

defineExpose({ self, focus });
</script>
<template>
  <div class="mx-auto mt-24 min-w-80 max-w-96 rounded border border-gray-200 bg-white px-9 py-7 text-gray-900">
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
      <HtmlInput
        v-model="slug"
        :icon="makeIcon({ faName: 'fas fa-at' })"
        name="Slug"
        title="Slug"
        :variant="Variant.PRIMARY"
        is-input
        is-disabled
      />
      <!-- Region -->
      <!-- ... -->
    </div>
    <!-- Actions -->
    <div class="mt-7">
      <Button
        v-if="state == 'create-bench'"
        name="Submit"
        :icon="makeIcon({ faName: 'fas fa-rocket-launch' })"
        title="Create Bench"
        class="w-full"
        :is-loading="isActive"
        :is-disabled="isActive"
        @click="submit"
      />
      <Button
        v-if="state == 'all-set'"
        name="Close"
        :icon="makeIcon({ faName: 'fas fa-xmark' })"
        title="Close"
        class="w-full"
        :variant="Variant.COMPACT"
        @click="() => canvas.removeView(spaceConnection.tx, spaceGraph, spaceGraph.get(self) as ViewData)"
      />
    </div>
  </div>
</template>
