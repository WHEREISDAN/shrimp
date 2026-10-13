import type { OnboardingStep } from "../../lib/ipc";

/** Onboarding's steps, in order (ONB-01..06). */
export const STEPS: OnboardingStep[] = [
  "welcome",
  "permissions",
  "microphone",
  "shortcut",
  "languages",
  "engine",
  "practice",
  "done",
];

export const nextStep = (step: OnboardingStep): OnboardingStep =>
  STEPS[Math.min(STEPS.indexOf(step) + 1, STEPS.length - 1)] ?? "done";

export const previousStep = (step: OnboardingStep): OnboardingStep =>
  STEPS[Math.max(STEPS.indexOf(step) - 1, 0)] ?? "welcome";
