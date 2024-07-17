<script lang="ts" setup>
import { ViewData, NodeReferenceData, Region, UserStatus, Variant } from "@/proto/wire/";
import { useExistingConnection } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { createBench, user } from "@/system/user";
import { viewEmits, type FocusAnchor } from "@/views/common";
import { toRef, type Ref, ref, watch, computed } from "vue";
import Button from "@/views/controls/Button.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { toNodeReference } from "@/proto/wiring";
import { canvas, goToBench } from "@/system/space";
import { getViewComponentChildren, isVueInstanceOf } from "@/views/canvas";

const props = defineProps<{ self: NodeReferenceData } & Pick<ViewData, "nodePtr">>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph } = useExistingConnection(toRef(props, "self"));

const self = toRef(props, "self");
const slug: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.EUROPE_CENTRAL);
const isActive = ref(false);
const isActivated = computed(() => user.value?.status == UserStatus.ACTIVATED);

// init slug with user slug
watch(
  user,
  () => {
    if (user.value) {
      slug.value = user.value.slug ?? "";
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
    return childViews.find((v) => isVueInstanceOf(v, NativeInput));
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
        <span v-if="isActivated">You already have a Bench.</span>
      </p>
    </div>
    <!-- Data -->
    <div v-if="!isActivated" class="mt-5">
      <!-- Owner -->
      <!-- ... -->
      <!-- Slug must match user slug for main bench -->
      <NativeInput
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
    <div v-if="!isActivated" class="mt-7">
      <Button
        name="Submit"
        :icon="makeIcon({ faName: 'fas fa-rocket-launch' })"
        title="Create Bench"
        class="w-full"
        :is-loading="isActive"
        :is-disabled="isActive"
        @click="submit"
      />
    </div>
  </div>
</template>
