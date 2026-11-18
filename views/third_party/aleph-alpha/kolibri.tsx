// Reasoning-effort text from the Aleph Alpha Kolibri-1 chat template, copied verbatim.
// Notice: ./NOTICE.md
"use strict_tokens";
const NO_REASONING = "Reasoning is disabled. Proceed straight to answering according to the user's instructions.";
const LOW_REASONING = "Reasoning effort is set to low. Think briefly through only the essential steps in the user's language, then proceed directly to the answer.";
const MEDIUM_REASONING = "Reasoning effort is set to medium. Think through the task methodically in the user's language, check key assumptions, and provide a well-supported answer.";
const HIGH_REASONING = "Reasoning effort is set to high. Think carefully through the task in the user's language, validate key assumptions, consider plausible alternatives, and prioritize correctness and clarity.";

export { HIGH_REASONING, LOW_REASONING, MEDIUM_REASONING, NO_REASONING };
