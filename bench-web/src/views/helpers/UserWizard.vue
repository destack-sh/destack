<script lang="ts" setup>
import { makeType } from "@/language/core/type";
import { BenchType, NodeType, Region, UserWizardViewStage, ViewData, type NodeReferenceData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { benchPtr } from "@/system/client";
import { canvas, goToBench } from "@/system/space";
import { logIn, signUp, user } from "@/system/user";
import { fireCommandById } from "@/ui/command";
import { makeIcon } from "@/ui/icon";
import { getViewComponentChildren, isVueInstanceOf } from "@/ui/view";
import { DEFAULT_REGION_BY_AREA, GEOLOCATION } from "@/utils/geolocation";
import { type FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import Picker from "@/views/content/Picker.vue";
import Button from "@/views/controls/Button.vue";
import ThreeIcon from "@/views/helpers/ThreeIcon.vue";
import { ref, toRef, watchEffect, type Ref } from "vue";

const props = defineProps<{ self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<ViewData, "title">>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

const stage = ref<UserWizardViewStage>(UserWizardViewStage.LOG_IN);
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.FRANKFURT);
watchEffect(() => {
  if (GEOLOCATION.value?.continent != null) {
    const defaultRegion = DEFAULT_REGION_BY_AREA[GEOLOCATION.value.continent];
    if (defaultRegion != null) {
      region.value = defaultRegion;
    }
  }
});
const isActive = ref(false);
const lastError: Ref<string | null> = ref(null);
const isCreating = ref(false);

function clear() {
  name.value = "";
  slug.value = "";
  email.value = "";
  password.value = "";
}

function switchStage() {
  if (stage.value == UserWizardViewStage.LOG_IN) {
    stage.value = UserWizardViewStage.SIGN_UP;
  } else if (stage.value == UserWizardViewStage.SIGN_UP) {
    stage.value = UserWizardViewStage.LOG_IN;
  } else {
    throw new Error(`unexpected registration stage: ${stage.value}`);
  }
}

async function submit() {
  isActive.value = true;
  lastError.value = null;
  try {
    if (stage.value == UserWizardViewStage.SIGN_UP) {
      isCreating.value = true;
      try {
        const { user } = await signUp(
          { name: name.value, slug: slug.value, email: email.value, region: region.value },
          password.value,
        );
        await goToBench({ bench: user.benchPtr as TypedNodeReferenceData<NodeType.BENCH> });
        await fireCommandById("space.create.page");
      } finally {
        isCreating.value = false;
      }
    } else if (stage.value == UserWizardViewStage.LOG_IN) {
      const { user } = await logIn({ slug: slug.value }, password.value);
      // if we're outside a Bench and have a Bench, go home
      if (user.benchPtr != null && benchPtr.value == null) {
        await goToBench({ bench: user.benchPtr as TypedNodeReferenceData<NodeType.BENCH> });
      }
    } else {
      throw new Error(`unexpected registration stage: ${stage.value}`);
    }
    clear();
  } catch (e) {
    lastError.value = (e as Error).message ?? "Unknown error";
  } finally {
    isActive.value = false;
  }
}

const instance = canvas.registerView(self, id);
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  const childViews = getViewComponentChildren(instance);
  if (anchor != "bottom") {
    return childViews.find((v) => isVueInstanceOf(v, NativeInput));
  } else {
    return childViews.reverse().find((v) => isVueInstanceOf(v, Button));
  }
}

defineExpose<ViewExpose>({ self, focus });
</script>
<template>
  <div class="flex h-full flex-row divide-x divide-gray-200">
    <!-- nocheckin -->
    <div class="flex h-full w-[70%] flex-col items-center justify-center bg-yellow-400">
      <ThreeIcon class="w-[20%] h-[20%]" />
    </div>

    <div class="h-full flex-1 rounded-sm px-9 py-7 text-gray-900">
      <div>
        <!-- Header -->
        <div>
          <h2 class="text-2xl font-semibold">{{ title }}</h2>
          <p class="mt-2 text-gray-500">
            <span v-if="stage == UserWizardViewStage.LOG_IN">Log into an existing Bench account.</span>
            <span v-else>Create a new Bench account.</span>
          </p>
        </div>
        <!-- Data -->
        <div class="mt-5 flex w-full flex-col gap-y-2">
          <div v-if="stage == UserWizardViewStage.SIGN_UP" class="flex flex-col gap-y-0.5">
            <span class="font-medium">Name</span>
            <NativeInput
              id="name"
              ref="nameRef"
              v-model="name"
              :icon="makeIcon({ faName: 'fas fa-user' })"
              name="Name"
              title="Name"
              is-input
            />
          </div>
          <div class="flex flex-col gap-y-0.5">
            <span class="font-medium">Username</span>
            <NativeInput
              id="slug"
              ref="slugRef"
              v-model="slug"
              :icon="makeIcon({ faName: 'fas fa-hashtag' })"
              name="slug"
              title="Username"
              is-input
            />
          </div>
          <div v-if="stage == UserWizardViewStage.SIGN_UP" class="flex flex-col gap-y-0.5">
            <span class="font-medium">Email</span>
            <NativeInput
              id="email"
              v-model="email"
              :icon="makeIcon({ faName: 'fas fa-at' })"
              name="Email"
              title="Email"
              is-input
            />
          </div>
          <!-- NOTE :UX: add passowrd feedback (see https://zxcvbn-ts.github.io/zxcvbn/)? -->
          <div class="flex flex-col gap-y-0.5">
            <span class="font-medium">Password</span>
            <NativeInput
              id="password"
              v-model="password"
              :icon="makeIcon({ faName: 'fas fa-key' })"
              name="Password"
              title="Password"
              is-input
              :value-type="makeType({ isSecret: true })"
            />
          </div>
          <div v-if="stage == UserWizardViewStage.SIGN_UP" class="flex flex-col gap-y-0.5">
            <span class="font-medium">Region</span>
            <Picker
              id="region"
              v-model="region"
              :icon="makeIcon({ faName: 'fas fa-globe' })"
              name="Region"
              title="Region"
              is-input
              :value-type="makeType({ benchType: BenchType.CONTINENT, isList: false, isRequired: true })"
            />
          </div>
        </div>
        <!-- Commands -->
        <div class="mt-7">
          <Button
            v-if="!user"
            id="submit"
            name="Submit"
            :icon="makeIcon({ faName: 'fas fa-arrow-right-from-bracket' })"
            :title="stage === UserWizardViewStage.LOG_IN ? 'Log in' : 'Sign up'"
            class="w-full"
            :is-disabled="isActive"
            :is-loading="isActive"
            @click="submit"
          />
          <Button
            v-if="!user"
            id="switch"
            name="Switch"
            :icon="makeIcon({ faName: 'fas fa-shuffle' })"
            :title="stage === UserWizardViewStage.LOG_IN ? 'Sign up' : 'Log in'"
            class="mt-2 w-full"
            @click="() => switchStage()"
          />
        </div>
        <!-- Error -->
        <div v-if="lastError" class="mt-3 flex w-full flex-col gap-y-1 border-t border-t-gray-200 pt-3">
          <div class="flex flex-row items-center gap-x-2">
            <i class="fas fa-circle-exclamation text-danger-600" />
            <span class="text-danger-600">Error</span>
          </div>
          <div class="flex flex-row items-center gap-x-2">
            <span class="text-gray-500">{{ lastError }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
