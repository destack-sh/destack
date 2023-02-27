export type SemVer = {
  major: number;
  minor: number;
  patch: number;
};

export function parseSemVer(version: string): SemVer | undefined {
  try {
    const [major, minor, patch] = version.split(".");
    return {
      major: parseInt(major),
      minor: parseInt(minor),
      patch: parseInt(patch),
    };
  } catch (e) {
    return undefined;
  }
}

export function renderSemVer(version: SemVer): string {
  return `${version.major}.${version.minor}.${version.patch}`;
}

export const FIRST_SEMVER: SemVer = { major: 0, minor: 0, patch: 0 };
export const FIRST_SEMVER_STRING = renderSemVer(FIRST_SEMVER);

export function bumpSemVer(version: SemVer, type: "major" | "minor" | "patch"): SemVer {
  switch (type) {
    case "major":
      return { major: version.major + 1, minor: 0, patch: 0 };
    case "minor":
      return { major: version.major, minor: version.minor + 1, patch: 0 };
    case "patch":
      return { major: version.major, minor: version.minor, patch: version.patch + 1 };
  }
}
