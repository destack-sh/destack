import type { Region } from "@/proto/wire";
import { IP_API_KEY } from "@/utils/globals";
import { log } from "@/utils/log";
import { createSharedComposable } from "@vueuse/core";
import { ref, shallowRef, type Ref } from "vue";

// see ip-api.com/docs
export interface GeolocationRaw {
  continent: string;
  continentCode: string;
  country: string;
  countryCode: string;
  countryCode3: string;
  region: string;
  regionName: string;
  city: string;
  lat: number;
  lon: number;
  isp: string;
  org: string;
  proxy: boolean;
  hosting: boolean;
  query: string;
}
const GEOLOCATION_FIELDS = [
  "status",
  "message",
  "continent",
  "continentCode",
  "country",
  "countryCode",
  "countryCode3",
  "region",
  "regionName",
  "city",
  "lat",
  "lon",
  "proxy",
  "hosting",
  "query",
];

export interface Geolocation {
  continent: Region;
  area: Region | undefined;
  city: Region | undefined;
  detail: GeolocationRaw;
}

// TODO :Security: move ip-api lookup behind proxy (to avoid exposing our token)
async function getGeolocation(): Promise<Geolocation> {
  try {
    const response = IP_API_KEY
      ? await fetch(`https://pro.ip-api.com/json/?fields=${GEOLOCATION_FIELDS.join(",")}&key=${IP_API_KEY}}`, {
          credentials: "omit",
        })
      : await fetch(`http://ip-api.com/json/?fields=${GEOLOCATION_FIELDS.join(",")}`); // only works in HTTP context (can't request HTTP in HTTPS)
    const data = await response.json();

    if (data.status === "fail") {
      throw new Error(data.message || "Failed to fetch geolocation");
    }
    log.trace("geolocation.complete", data);

    return data as Geolocation;
  } catch (error) {
    log.error("geolocation.error", error);
    throw error;
  }
}

const _geolocation: Ref<Geolocation | null> = shallowRef(null);

function _useGeolocation() {
  const loading = ref(false);
  const error = ref<Error | null>(null);

  const fetchLocation = async () => {
    loading.value = true;
    error.value = null;

    try {
      _geolocation.value = await getGeolocation();
    } catch (err) {
      error.value = err instanceof Error ? err : new Error("An unknown error occurred");
    } finally {
      loading.value = false;
    }
  };

  const refreshLocation = async () => {
    await fetchLocation();
  };

  fetchLocation();

  return {
    location: _geolocation,
    loading,
    error,
    refreshLocation,
  };
}

export const useGeolocation = createSharedComposable(_useGeolocation);
export const GEOLOCATION = useGeolocation().location;
