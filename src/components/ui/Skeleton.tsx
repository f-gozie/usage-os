import { cn } from "@/lib/utils";

export interface SkeletonProps {
  className?: string;
  /** Render as a circle (e.g. the dial placeholder). */
  circle?: boolean;
}

/** Shimmering placeholder for the loading state. The `.skeleton` class (index.css)
 *  carries the theme-aware gradient + animation (paused under reduced-motion). */
export function Skeleton({ className, circle = false }: SkeletonProps) {
  return <div className={cn("skeleton", circle && "rounded-full", className)} />;
}
