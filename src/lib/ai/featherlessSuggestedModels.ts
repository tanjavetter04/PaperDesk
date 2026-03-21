/**
 * Ten general-purpose chat / writing models on Featherless (checked against
 * `GET https://api.featherless.ai/v1/models`). IDs must match the API exactly.
 */
export const FEATHERLESS_SUGGESTED_MODELS = [
  {
    id: "meta-llama/Meta-Llama-3.1-8B-Instruct",
    labelDe: "Llama 3.1 — 8B (schnell, Standard)",
    labelEn: "Llama 3.1 — 8B (fast, default)",
  },
  {
    id: "meta-llama/Meta-Llama-3.1-70B-Instruct",
    labelDe: "Llama 3.1 — 70B (stark)",
    labelEn: "Llama 3.1 — 70B (strong)",
  },
  {
    id: "mistralai/Mistral-Large-Instruct-2411",
    labelDe: "Mistral Large",
    labelEn: "Mistral Large",
  },
  {
    id: "mistralai/Mistral-Small-24B-Instruct-2501",
    labelDe: "Mistral Small 24B",
    labelEn: "Mistral Small 24B",
  },
  {
    id: "mistralai/Mistral-Small-3.2-24B-Instruct-2506",
    labelDe: "Mistral Small 3.2 — 24B",
    labelEn: "Mistral Small 3.2 — 24B",
  },
  {
    id: "Qwen/Qwen2.5-72B-Instruct",
    labelDe: "Qwen 2.5 — 72B",
    labelEn: "Qwen 2.5 — 72B",
  },
  {
    id: "Qwen/Qwen2.5-32B-Instruct",
    labelDe: "Qwen 2.5 — 32B",
    labelEn: "Qwen 2.5 — 32B",
  },
  {
    id: "google/gemma-2-27b-it",
    labelDe: "Gemma 2 — 27B",
    labelEn: "Gemma 2 — 27B",
  },
  {
    id: "deepseek-ai/DeepSeek-R1-Distill-Llama-70B",
    labelDe: "DeepSeek R1 (Distill Llama 70B)",
    labelEn: "DeepSeek R1 (distilled Llama 70B)",
  },
  {
    id: "NousResearch/Hermes-3-Llama-3.1-8B",
    labelDe: "Hermes 3 — Llama 3.1 8B",
    labelEn: "Hermes 3 — Llama 3.1 8B",
  },
] as const;

export type FeatherlessSuggestedModel = (typeof FEATHERLESS_SUGGESTED_MODELS)[number];
