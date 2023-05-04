import animalsString from "@/assets/animals.txt?raw";
import adjectivesString from "@/assets/adjectives.txt?raw";

const animals = animalsString.toLowerCase().split("\n");
const adjectives = adjectivesString.toLowerCase().split("\n");

function getRandomElement<T>(array: T[]): T {
  const randomIndex = Math.floor(Math.random() * array.length);
  return array[randomIndex];
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export function getRandomAdjective() {
  return getRandomElement(adjectives);
}

export function getRandomName() {
  const randomAdjective = getRandomElement(adjectives);
  const randomNoun = getRandomElement(animals);
  // capitalize first letters
  return `${capitalize(randomAdjective)} ${capitalize(randomNoun)}`;
}
