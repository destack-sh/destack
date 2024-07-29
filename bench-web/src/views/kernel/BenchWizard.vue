<script lang="ts" setup>
import {
  BenchType,
  EnumType,
  NodeReferenceData,
  Region,
  UserStatus,
  Variant,
  ViewData
} from "@/proto/wire/";
import { toPlainNodeRef } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { enumIndex } from "@/system/search";
import { canvas, goToBench } from "@/system/space";
import { createBench, user } from "@/system/user";
import { makeTypeInfo } from "@/system/value";
import { DEFAULT_REGION_BY_AREA, GEOLOCATION } from "@/utils/geolocation";
import { getViewComponentChildren, isVueInstanceOf } from "@/views/canvas";
import { viewEmits, type FocusAnchor } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import Picker from "@/views/content/Picker.vue";
import Button from "@/views/controls/Button.vue";
import { computed, ref, toRef, watch, watchEffect, type Ref } from "vue";

const props = defineProps<{ self: NodeReferenceData } & Pick<ViewData, "nodePtr">>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph } = useExistingConnection(toRef(props, "self"));

const self = toRef(props, "self");
const slug: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.FRANKFURT);
watchEffect(() => {
  if (GEOLOCATION.value?.area != null) {
    const defaultRegion = DEFAULT_REGION_BY_AREA[GEOLOCATION.value.area];
    if (defaultRegion != null) {
      region.value = defaultRegion;
    }
  }
});
const isActive = ref(false);
const isActivated = computed(() => user.value?.status == UserStatus.ACTIVATED);
const lastError: Ref<string | null> = ref(null);

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
  lastError.value = null;
  try {
    if (!user.value) throw new Error("no active user");
    const { bench } = await createBench({
      owner: toPlainNodeRef(user.value),
      slug: slug.value,
      region: region.value,
      isMain: true,
    });
    await goToBench({ bench: toPlainNodeRef(bench) });
  } catch (e) {
    lastError.value = (e as Error).message ?? "Unknown error";
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
    <div v-if="!isActivated" class="mt-5 flex flex-col gap-y-3">
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
      <!-- Region Area -->
      <Picker
        v-model="region"
        :icon="makeIcon({ faName: 'fas fa-globe' })"
        name="Region"
        title="Region"
        is-input
        :value-type="makeTypeInfo({ benchType: BenchType.REGION, isList: false })"
        :custom-index="
          enumIndex({
            id: 'geolocation',
            enumTypes: [EnumType.REGION],
            enumValues: [Region.FRANKFURT, Region.OHIO],
          })
        "
      />
    </div>
    <!-- Actions -->
    <div v-if="!isActivated" class="mt-7">
      <Button
        name="Submit"
        :icon="makeIcon({ faName: 'fas fa-plus' })"
        title="Create Bench"
        class="w-full"
        :is-loading="isActive"
        :is-disabled="isActive"
        @click="submit"
      />
    </div>
    <!-- Error -->
    <div v-if="lastError" class="mt-3 flex w-full flex-col gap-y-1 border-t border-t-gray-200 pt-3">
      <div class="flex flex-row items-center gap-x-2">
        <i class="fas fa-exclamation-triangle text-danger-600" />
        <span class="text-danger-600">Error</span>
      </div>
      <div class="flex flex-row items-center gap-x-2">
        <span class="text-gray-500">{{ lastError }}</span>
      </div>
    </div>
  </div>
</template>
