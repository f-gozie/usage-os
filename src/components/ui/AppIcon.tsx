import { useEffect, useState } from 'react';

import { loadIconMap, resolveIcon } from '@/lib/appIcons';
import { cn } from '@/lib/utils';

export interface AppIconProps {
  /** The app / process name to resolve (e.g. "Google Chrome", "iTerm2"). */
  name: string;
  /** Edge length in px (default 16). */
  size?: number;
  className?: string;
}

/** A small app icon resolved from the user's installed apps by name, with a monogram
 *  fallback (first letter on the ink colour) for names that don't resolve — e.g. the
 *  app's own dev binary. The icon map loads once and is shared; the same data-URI `src`
 *  is reused across rows, so the webview decodes one copy regardless of count. */
export function AppIcon({ name, size = 16, className }: AppIconProps) {
  // Resolve synchronously when the map is already loaded (no flash when re-entering a
  // view); otherwise load it and update. `undefined` means "not loaded yet".
  const [icon, setIcon] = useState<string | null>(() => resolveIcon(name) ?? null);

  useEffect(() => {
    const ready = resolveIcon(name);
    if (ready !== undefined) {
      setIcon(ready);
      return;
    }
    let alive = true;
    void loadIconMap().then(() => {
      if (alive) setIcon(resolveIcon(name) ?? null);
    });
    return () => {
      alive = false;
    };
  }, [name]);

  const dim = { width: `${size}px`, height: `${size}px` };

  if (icon) {
    return (
      <img
        src={icon}
        alt=""
        aria-hidden
        className={cn('shrink-0 rounded-[4px] object-cover', className)}
        style={dim}
      />
    );
  }

  const letter = name.replace(/[^a-z0-9]/gi, '').charAt(0).toUpperCase() || '?';
  return (
    <span
      aria-hidden
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded-[4px] font-bold leading-none',
        className,
      )}
      style={{
        ...dim,
        fontSize: `${Math.round(size * 0.56)}px`,
        background: 'var(--edge)',
        color: 'var(--bg)',
      }}
    >
      {letter}
    </span>
  );
}
