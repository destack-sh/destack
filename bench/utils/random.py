import random

ADJECTIVES = [
    "amazing",
    "beautiful",
    "colorful",
    "crunchy",
    "delicious",
    "delightful",
    "excellent",
    "fantastic",
    "fresh",
    "happy",
    "hopeful",
    "hopeless",
    "juicy",
    "magical",
    "magnificent",
    "marvelous",
    "perfect",
    "ripe",
    "shiny",
    "sour",
    "sparkling",
    "splendid",
    "sweet",
    "tart",
    "tasty",
    "wonderful",
    "yummy",
]

OBJECTS = [
    "artichoke",
    "apple",
    "avocado",
    "banana",
    "blueberry",
    "broccoli",
    "cabbage",
    "carrot",
    "coconut",
    "date",
    "dragonfruit",
    "eggplant",
    "elderberry",
    "fig",
    "grape",
    "kiwi",
    "lemon",
    "mango",
    "orange",
    "potato",
    "quince",
    "raspberry",
    "strawberry",
    "tomato",
    "turnip",
    "watermelon",
    "yam",
    "zucchini",
]


def get_random_veggie_name():
    adjective = random.choice(ADJECTIVES)
    fruit_vegetable = random.choice(OBJECTS)

    return f"{adjective.capitalize()} {fruit_vegetable.capitalize()}"
