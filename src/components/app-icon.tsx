import appIcon from "../../assets/icons/app-icon.svg";
import { cn } from "@/lib/utils";

/** Shrimp's app icon, as in the Dock (with Apple's grid margin around the squircle). */
export function AppIcon({ className }: { className?: string | undefined }) {
  return (
    <img
      src={appIcon}
      alt=""
      aria-hidden="true"
      draggable={false}
      className={cn("size-16 select-none", className)}
    />
  );
}
