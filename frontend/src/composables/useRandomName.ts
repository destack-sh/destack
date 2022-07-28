import animalsString from "@/assets/animals.txt?raw";
import adjectivesString from "@/assets/adjectives.txt?raw";

const animals = animalsString
  .toLowerCase()
  .split("\n")
  .map((s) => s.replace(" ", "-"));
const adjectives = adjectivesString
  .toLowerCase()
  .split("\n")
  .map((s) => s.replace(" ", "-"));

function getRandomElement<T>(array: T[]): T {
  const randomIndex = Math.floor(Math.random() * array.length);
  return array[randomIndex];
}

export function getRandomName() {
  const randomAdjective = getRandomElement(adjectives);
  const randomNoun = getRandomElement(animals);
  return `${randomAdjective}-${randomNoun}`;
}
