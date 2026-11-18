"""Oolong-style long-context aggregation tasks (after Oolong-synth): a log of user messages, one
per line, whose category (complaint, praise, question, request) is never written down; a question
aggregates over the categories, so answering means reading every line, not finding one. Some
phrasings defeat keyword rules (a polite complaint, praise phrased as a question, a question with
no question mark).

  python3 make_tasks.py SEED N_MESSAGES > tasks.json

Prints a JSON list of {id, question, context, expect} (`expect`: the exact answer, as a string).
"""
import json, random, sys

TEMPLATES = {
    "complaint": [
        "The {thing} stopped working again after the update, this is the third time this week.",
        "I waited {n} days for the {thing} and it arrived broken.",
        "Honestly disappointed: the {thing} is slower than the old one.",
        "Your support line kept me on hold for {n} minutes and then hung up.",
        "I was charged twice for the {thing} and nobody has fixed it.",
        "The {thing} keeps crashing whenever I open it.",
        "Please fix the {thing}, it reboots every hour and I lose my work.",
        "Why does the {thing} keep dropping the connection? This is unacceptable.",
        "Thanks for nothing: the {thing} you replaced fails the same way.",
    ],
    "praise": [
        "The new {thing} is fantastic, setup took {n} minutes.",
        "Big thanks to the team, the {thing} works perfectly now.",
        "Really impressed with how quickly support sorted out my {thing}.",
        "Love the {thing}, best purchase I made this year.",
        "The {thing} exceeded my expectations, great job.",
        "Smooth experience with the {thing} from start to finish.",
        "Is it just me or is the new {thing} amazing?",
        "Could not be happier with the {thing}, honestly.",
        "No complaints at all about the {thing}, it just works.",
    ],
    "question": [
        "Does the {thing} work with the older model?",
        "How long is the warranty on the {thing}?",
        "Can I use the {thing} abroad without an adapter?",
        "Is there a way to export my data from the {thing}?",
        "What does the blinking light on the {thing} mean?",
        "Will the {thing} get the update this month?",
        "I was wondering whether the {thing} supports dark mode.",
        "Not sure if the {thing} needs a hub, can someone tell me.",
        "Curious what the battery life of the {thing} is in practice.",
    ],
    "request": [
        "Please send me an invoice for the {thing}.",
        "Could you cancel my order for the {thing}?",
        "I'd like to upgrade my plan to include the {thing}.",
        "Please change the delivery address for my {thing}.",
        "Kindly add a second user to my {thing} account.",
        "Please schedule a technician visit for the {thing}.",
        "When you get a chance, send over the manual for the {thing}.",
        "Can you move my {thing} renewal to next month?",
        "Go ahead and ship the replacement {thing} to my office.",
    ],
}
OPENERS = ["", "", "Hi team, ", "Quick note: ", "Hello! ", "Hey, "]
THINGS = ["router", "thermostat", "headset", "camera", "app", "printer", "smart plug", "speaker", "tablet", "doorbell"]


def make(seed, n):
    rng = random.Random(seed)
    users = list(range(100, 140))
    rows = []
    for i in range(n):
        cat = rng.choice(list(TEMPLATES))
        text = rng.choice(OPENERS) + rng.choice(TEMPLATES[cat]).format(thing=rng.choice(THINGS), n=rng.randint(2, 45))
        rows.append((f"2026-{rng.randint(1, 9):02d}-{rng.randint(1, 28):02d}", rng.choice(users), cat, text))
    context = "\n".join(f"Date: {d} || User: {u} || Message: {t}" for d, u, _, t in rows)
    count = lambda pred: sum(1 for r in rows if pred(r))
    tasks = [{"question": "Each line of the context is a message from a user. How many of the messages are complaints? Answer with the number only.",
              "expect": str(count(lambda r: r[2] == "complaint"))}]
    praise = {u: count(lambda r, u=u: r[1] == u and r[2] == "praise") for u in users}
    best = max(praise.values())
    if list(praise.values()).count(best) == 1:
        tasks.append({"question": "Each line of the context is a message from a user. Which user wrote the most messages praising the product or service? Answer with the user id only.",
                      "expect": str(max(praise, key=praise.get))})
    u = rng.choice(users)
    tasks.append({"question": f"Each line of the context is a message from a user. How many of user {u}'s messages ask a question? Answer with the number only.",
                  "expect": str(count(lambda r: r[1] == u and r[2] == "question"))})
    return [{"id": f"s{seed}-q{k}", "context": context, **t} for k, t in enumerate(tasks)]


if __name__ == "__main__":
    print(json.dumps(make(int(sys.argv[1]), int(sys.argv[2]))))
