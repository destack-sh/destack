/* eslint-disable no-console */
import { Area, Continent, Region } from "@/proto/wire";
import { IP_API_KEY } from "@/utils/globals";
import { log } from "@/utils/log";
import { formatDuration } from "@/utils/time";
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
  continent: Continent | null;
  area: Area | null;
  country: string | null;
  detail: GeolocationRaw;
}

// TODO :Security: move ip-api lookup behind proxy (to avoid exposing our token)
async function getGeolocation(): Promise<Geolocation> {
  try {
    const startedAt = performance.now();
    const response = IP_API_KEY
      ? await fetch(`https://pro.ip-api.com/json/?fields=${GEOLOCATION_FIELDS.join(",")}&key=${IP_API_KEY}`, {
          credentials: "omit",
        })
      : await fetch(`http://ip-api.com/json/?fields=${GEOLOCATION_FIELDS.join(",")}`); // only works in HTTP context (can't request HTTP in HTTPS)
    const data = await response.json();
    if (data.status === "fail") {
      throw new Error(data.message || "Failed to fetch geolocation");
    }

    const duration = performance.now() - startedAt;
    const geolocation = parseGeolocation(data);
    log.debug("geolocation.complete", formatDuration(duration), data);
    console.group(`%cgeo`, "color:yellow");
    console.info(
      `%cCONTINENT: ${geolocation.continent ? Continent[geolocation.continent] : "<unknown>"}`,
      "color:yellow",
    );
    console.info(`%cAREA: ${geolocation.area ? Area[geolocation.area] : "<unknown>"}`, "color:yellow");
    console.info(`%cCOUNTRY: ${geolocation.country}`, "color:yellow");
    console.groupEnd();
    return geolocation;
  } catch (error) {
    log.error("geolocation.error", error);
    throw error;
  }
}

function parseGeolocation(data: GeolocationRaw): Geolocation {
  const continent = CONTINENT_BY_CONTINENT_CODE[data.continentCode];
  const area = AREA_BY_COUNTRY_CODE[data.countryCode];
  const gelocation: Geolocation = { continent: continent, area: area, country: data.countryCode, detail: data };
  return gelocation;
}

const CONTINENT_BY_CONTINENT_CODE: Record<string, Continent> = {
  EU: Continent.EUROPE,
  NA: Continent.NORTH_AMERICA,
  SA: Continent.SOUTH_AMERICA,
  AS: Continent.ASIA,
  AF: Continent.AFRICA,
  OC: Continent.AUSTRALIA,
  AU: Continent.AUSTRALIA,
};

const AREA_BY_COUNTRY_CODE: Record<string, Area> = {
  // eu-central
  AT: Area.EUROPE_CENTRAL,
  BE: Area.EUROPE_CENTRAL,
  CH: Area.EUROPE_CENTRAL,
  CZ: Area.EUROPE_CENTRAL,
  DE: Area.EUROPE_CENTRAL,
  DK: Area.EUROPE_CENTRAL,
  ES: Area.EUROPE_CENTRAL,
  FI: Area.EUROPE_CENTRAL,
  FR: Area.EUROPE_CENTRAL,
  GB: Area.EUROPE_CENTRAL,
  IE: Area.EUROPE_CENTRAL,
  IT: Area.EUROPE_CENTRAL,
  NL: Area.EUROPE_CENTRAL,
  NO: Area.EUROPE_CENTRAL,
  PL: Area.EUROPE_CENTRAL,
  PT: Area.EUROPE_CENTRAL,
  SE: Area.EUROPE_CENTRAL,
  // na-east
  US: Area.NORTH_AMERICA_EAST,
  CA: Area.NORTH_AMERICA_EAST,
  // na-west
  MX: Area.NORTH_AMERICA_WEST,
  // sa-east
  AR: Area.SOUTH_AMERICA_EAST,
  BR: Area.SOUTH_AMERICA_EAST,
  CL: Area.SOUTH_AMERICA_EAST,
  CO: Area.SOUTH_AMERICA_EAST,
  PE: Area.SOUTH_AMERICA_EAST,
  // me-central
  AE: Area.MIDDLE_EAST_CENTRAL,
  BH: Area.MIDDLE_EAST_CENTRAL,
  IL: Area.MIDDLE_EAST_CENTRAL,
  KW: Area.MIDDLE_EAST_CENTRAL,
  OM: Area.MIDDLE_EAST_CENTRAL,
  QA: Area.MIDDLE_EAST_CENTRAL,
  SA: Area.MIDDLE_EAST_CENTRAL,
  // me-west
  EG: Area.MIDDLE_EAST_WEST,
  JO: Area.MIDDLE_EAST_WEST,
  LB: Area.MIDDLE_EAST_WEST,
  TR: Area.MIDDLE_EAST_WEST,
  // af-south
  ZA: Area.AFRICA_SOUTH,
  NG: Area.AFRICA_SOUTH,
  KE: Area.AFRICA_SOUTH,
  // as-west
  RU: Area.ASIA_WEST,
  KZ: Area.ASIA_WEST,
  // as-south
  IN: Area.ASIA_SOUTH,
  PK: Area.ASIA_SOUTH,
  BD: Area.ASIA_SOUTH,
  LK: Area.ASIA_SOUTH,
  // as-east
  CN: Area.ASIA_EAST,
  HK: Area.ASIA_EAST,
  JP: Area.ASIA_EAST,
  KR: Area.ASIA_EAST,
  MY: Area.ASIA_EAST,
  PH: Area.ASIA_EAST,
  SG: Area.ASIA_EAST,
  TH: Area.ASIA_EAST,
  TW: Area.ASIA_EAST,
  VN: Area.ASIA_EAST,
  // au-south
  AU: Area.AUSTRALIA_SOUTH,
  NZ: Area.AUSTRALIA_SOUTH,
};

export const DEFAULT_REGION_BY_AREA: Partial<Record<Continent, Region>> = {
  [Continent.EUROPE]: Region.FRANKFURT,
  [Continent.NORTH_AMERICA]: Region.VIRGINIA,
  [Continent.SOUTH_AMERICA]: Region.SAO_PAULO,
  [Continent.AFRICA]: Region.CAPE_TOWN,
  [Continent.ASIA]: Region.MUMBAI,
  [Continent.AUSTRALIA]: Region.SYDNEY,
};

export const DEFAULT_REGION_BY_CONTINENT: Partial<Record<Continent, Region>> = {
  [Continent.EUROPE]: Region.FRANKFURT,
  [Continent.NORTH_AMERICA]: Region.VIRGINIA,
  [Continent.SOUTH_AMERICA]: Region.SAO_PAULO,
  [Continent.AFRICA]: Region.CAPE_TOWN,
  [Continent.ASIA]: Region.SINGAPORE,
  [Continent.AUSTRALIA]: Region.SYDNEY,
};

export const REGION_CONTINENT_SLUGS: Partial<Record<Continent, string>> = {
  [Continent.EUROPE]: "eu",
  [Continent.NORTH_AMERICA]: "na",
  [Continent.SOUTH_AMERICA]: "sa",
  [Continent.MIDDLE_EAST]: "me",
  [Continent.AFRICA]: "af",
  [Continent.ASIA]: "as",
  [Continent.AUSTRALIA]: "au",
};

export const REGION_SLUGS: Partial<Record<Region, string>> = {
  [Region.ZURICH]: "eu-zurich",
  [Region.FRANKFURT]: "eu-frankfurt",
  [Region.VIRGINIA]: "na-virginia",
  [Region.SAO_PAULO]: "sa-sao-paulo",
  [Region.CAPE_TOWN]: "af-cape-town",
  [Region.MUMBAI]: "as-mumbai",
  [Region.SINGAPORE]: "as-singapore",
  [Region.SYDNEY]: "au-sydney",
};

export function getRegionSlug(region: Region): string {
  return REGION_SLUGS[region] ?? Region[region].toLowerCase().replace("_", "-");
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
