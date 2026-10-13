import type { AppCategory, Style } from "../lib/ipc";

export const STYLE_TEXT: Record<Style, { title: string; description: string }> = {
  formal: { title: "Formal.", description: "Caps + punctuation." },
  casual: { title: "Casual", description: "Caps + less punctuation." },
  veryCasual: { title: "very casual", description: "No caps + less punctuation." },
  excited: { title: "Excited!", description: "More exclamations." },
};

interface CategoryInfo {
  id: AppCategory;
  label: string;
  /** Whether `apps` is a list of names rather than a sentence. */
  appList: boolean;
  apps: string;
  /** Same words in every style; only capitals and punctuation change. */
  examples: Partial<Record<Style, string>>;
}

export const CATEGORIES: CategoryInfo[] = [
  {
    id: "personal",
    label: "Personal messages",
    appList: true,
    apps: "Messages, WhatsApp, Discord, Signal, Telegram, Instagram",
    examples: {
      formal: "Hey, are you free for lunch tomorrow? Let's do 12 if that works.",
      casual: "Hey are you free for lunch tomorrow? Let's do 12 if that works",
      veryCasual: "hey are you free for lunch tomorrow? let's do 12 if that works",
    },
  },
  {
    id: "work",
    label: "Work messages",
    appList: true,
    apps: "Slack, Microsoft Teams, LinkedIn",
    examples: {
      formal: "Thanks for the update. The numbers look good. Let's go with the first option.",
      casual: "Thanks for the update, the numbers look good. Let's go with the first option",
      excited: "Thanks for the update! The numbers look good! Let's go with the first option!",
    },
  },
  {
    id: "email",
    label: "Email",
    appList: true,
    apps: "Mail, Outlook, Missive, Superhuman, Spark, Thunderbird, Mimestream",
    examples: {
      formal:
        "Hi Dana, thanks for sending the contract over. I'll review it today and get back to you by Friday.",
      casual:
        "Hi Dana thanks for sending the contract over, I'll review it today and get back to you by Friday",
      excited:
        "Hi Dana! Thanks for sending the contract over! I'll review it today and get back to you by Friday!",
    },
  },
  {
    id: "other",
    label: "Other",
    appList: false,
    apps: "Everything else, including AI assistants and terminals.",
    examples: {
      formal: "The build finished and the tests all passed, so I'm merging it now.",
      casual: "The build finished and the tests all passed so I'm merging it now",
      excited: "The build finished and the tests all passed so I'm merging it now!",
    },
  },
];
