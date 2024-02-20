export type SemVer = {
  major: number;
  minor: number;
  patch: number;
};

export function parseSemVer(version: string): SemVer | undefined {
  try {
    const [major, minor, patch] = version.split(".");
    const semVer = {
      major: parseInt(major),
      minor: parseInt(minor),
      patch: parseInt(patch),
    };
    // check if they're all valid integers
    if (isNaN(semVer.major) || isNaN(semVer.minor) || isNaN(semVer.patch)) {
      return undefined;
    }
    return semVer;
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

export function isSemVerNewer(a: SemVer, b: SemVer): boolean {
  if (a.major > b.major) {
    return true;
  } else if (a.major == b.major) {
    if (a.minor > b.minor) {
      return true;
    } else if (a.minor == b.minor) {
      if (a.patch > b.patch) {
        return true;
      }
    }
  }
  return false;
}
