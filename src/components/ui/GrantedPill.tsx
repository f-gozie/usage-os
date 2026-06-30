/** The "Granted ✓" state for a permission row — shared by onboarding and Settings. `label`
 *  overrides the text (e.g. "Enabled ✓" for the update opt-in). */
export function GrantedPill({ label = "Granted ✓" }: { label?: string } = {}) {
  return (
    <span className="whitespace-nowrap border-2 border-edge bg-bar-bg px-[11px] py-[5px] text-[11px] font-semibold uppercase tracking-[0.06em] text-bar-fg">
      {label}
    </span>
  );
}
