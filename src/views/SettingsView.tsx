import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useMemo, useState } from "react";

import { ContextEditorModal } from "@/components/settings/ContextEditorModal";
import { DeleteAllModal } from "@/components/settings/DeleteAllModal";
import { ExclusionModal } from "@/components/settings/ExclusionModal";
import {
  AddRow,
  IconButton,
  ModePill,
  Pill,
  SettingGroup,
  SettingRow,
  Tag,
} from "@/components/settings/primitives";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { Select } from "@/components/ui/Select";
import { Skeleton } from "@/components/ui/Skeleton";
import { useSettingsData } from "@/hooks/useSettingsData";
import { contextColorVar } from "@/lib/contexts";
import {
  deleteExclusion,
  exportEventsCsv,
  getDatabasePath,
  setRetentionDays,
  type Context,
  type Rule,
} from "@/lib/tauri";
import { THEMES, THEME_LABELS, useTheme, type Theme } from "@/providers/ThemeProvider";

const CANON_ORDER = ["deep", "research", "comms", "breaks"];

const RETENTION: ReadonlyArray<readonly [string, string]> = [
  ["30", "30 days"],
  ["90", "90 days"],
  ["365", "1 year"],
  ["0", "Forever"],
];

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : "Something went wrong.";
}

export function SettingsView() {
  const { data, loading, error, refresh } = useSettingsData();
  const { theme, setTheme } = useTheme();

  const [editing, setEditing] = useState<{ context: Context | null } | null>(null);
  const [exclusionOpen, setExclusionOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  // Auto-dismiss the action notice.
  useEffect(() => {
    if (!notice) return;
    const id = setTimeout(() => setNotice(null), 4500);
    return () => clearTimeout(id);
  }, [notice]);

  const contexts = useMemo(() => {
    const list = data?.contexts ?? [];
    return [...list].sort((a, b) => {
      const ai = a.slug ? CANON_ORDER.indexOf(a.slug) : 99;
      const bi = b.slug ? CANON_ORDER.indexOf(b.slug) : 99;
      return ai !== bi ? ai - bi : a.name.localeCompare(b.name);
    });
  }, [data?.contexts]);

  if (loading && !data) return <SettingsSkeleton />;
  if (error && !data) {
    return (
      <div className="flex flex-col items-center gap-3 border-2 border-dashed border-edge px-6 py-20 text-center">
        <p className="text-sm font-medium text-muted">{error}</p>
        <Pill onClick={() => void refresh()}>Retry</Pill>
      </div>
    );
  }
  if (!data) return null;

  const { rules, exclusions, settings } = data;

  const currentRetention = settings["data_retention_days"] ?? "0";
  const retentionOptions = RETENTION.some(([v]) => v === currentRetention)
    ? RETENTION.map(([value, label]) => ({ value, label }))
    : [
        ...RETENTION.map(([value, label]) => ({ value, label })),
        { value: currentRetention, label: `${currentRetention} days` },
      ];

  const onRetention = async (value: string) => {
    try {
      const removed = await setRetentionDays(Number(value));
      await refresh();
      setNotice(
        removed > 0
          ? `Removed ${removed} entr${removed === 1 ? "y" : "ies"} past the new window.`
          : "Retention window updated.",
      );
    } catch (e) {
      setNotice(errMsg(e));
    }
  };

  const revealDb = async () => {
    try {
      await revealItemInDir(await getDatabasePath());
    } catch (e) {
      setNotice(errMsg(e));
    }
  };

  const exportCsv = async () => {
    try {
      const path = await exportEventsCsv();
      setNotice(`Exported to ${path.split("/").pop() ?? path}.`);
      await revealItemInDir(path);
    } catch (e) {
      setNotice(errMsg(e));
    }
  };

  const removeExclusion = async (id: number) => {
    try {
      await deleteExclusion(id);
      await refresh();
    } catch (e) {
      setNotice(errMsg(e));
    }
  };

  return (
    <div>
      <p
        aria-live="polite"
        className="mb-3 min-h-[18px] text-[11px] font-semibold uppercase tracking-[0.08em] text-c-deep"
      >
        {notice}
      </p>

      {/* Contexts & rules */}
      <SettingGroup title="Contexts & rules">
        {contexts.map((ctx) => (
          <ContextRow
            key={ctx.id}
            ctx={ctx}
            rules={rules}
            onEdit={() => setEditing({ context: ctx })}
          />
        ))}
        <AddRow
          label="+ Add context"
          onAdd={() => setEditing({ context: null })}
          hint="Rules sort your activity automatically. Edit one and everything re-sorts."
        />
      </SettingGroup>

      {/* Privacy & exclusions */}
      <SettingGroup title="Privacy & exclusions">
        {exclusions.map((ex) => (
          <div key={ex.id} className="flex items-center gap-3 px-4 py-[13px]">
            <Tag>{ex.match_type}</Tag>
            <span className="flex-1 text-sm font-semibold">
              {ex.pattern}
              {ex.mode === "private" && (
                <span className="ml-1.5 text-xs font-medium text-muted">— title hidden</span>
              )}
            </span>
            <ModePill mode={ex.mode} />
            <IconButton aria-label={`Remove ${ex.pattern}`} onClick={() => void removeExclusion(ex.id)}>
              ×
            </IconButton>
          </div>
        ))}
        <div className="flex items-center gap-3 px-4 py-[13px]">
          <Tag>System</Tag>
          <span className="flex-1 text-sm font-semibold">
            Incognito / private windows
            <span className="ml-1.5 text-xs font-medium text-muted">— never recorded</span>
          </span>
          <span className="flex items-center gap-1.5 text-[10.5px] font-semibold uppercase tracking-[0.06em] text-muted">
            🔒 Always on
          </span>
        </div>
        <AddRow
          label="+ Add exclusion"
          onAdd={() => setExclusionOpen(true)}
          hint="Exclude leaves an app out completely. Private still counts the time, no titles."
        />
      </SettingGroup>

      {/* Appearance */}
      <SettingGroup title="Appearance">
        <SettingRow
          label="Theme"
          description="Choose a light or dark look. Warm is a softer dark; Black is a deeper one."
        >
          <SegmentedControl
            aria-label="Theme"
            value={theme}
            onChange={(v) => setTheme(v as Theme)}
            options={THEMES.map((t) => ({ value: t, label: THEME_LABELS[t] }))}
          />
        </SettingRow>
      </SettingGroup>

      {/* Your data */}
      <SettingGroup title="Your data">
        <SettingRow
          label="Where your data lives"
          description="One file on your Mac. Nothing is stored anywhere else."
        >
          <Pill onClick={() => void revealDb()}>Show in Finder</Pill>
        </SettingRow>
        <SettingRow
          label="Keep history for"
          description="Anything older than this is deleted automatically. You decide how much to keep."
        >
          <Select
            aria-label="Keep history for"
            value={currentRetention}
            onChange={(v) => void onRetention(v)}
            options={retentionOptions}
          />
        </SettingRow>
        <SettingRow label="Export" description="Take everything with you as CSV — it's your data.">
          <Pill onClick={() => void exportCsv()}>Export CSV</Pill>
        </SettingRow>
        <SettingRow
          label="Delete all data"
          danger
          description="Erase everything UsageOS has recorded. This can't be undone."
        >
          <Pill danger onClick={() => setDeleteOpen(true)}>
            Delete
          </Pill>
        </SettingRow>
      </SettingGroup>

      <ContextEditorModal
        open={editing !== null}
        context={editing?.context ?? null}
        rules={rules}
        onClose={() => setEditing(null)}
        onSaved={refresh}
      />
      <ExclusionModal
        open={exclusionOpen}
        onClose={() => setExclusionOpen(false)}
        onSaved={refresh}
      />
      <DeleteAllModal
        open={deleteOpen}
        onClose={() => setDeleteOpen(false)}
        onDeleted={async () => {
          await refresh();
          setNotice("All recorded activity deleted.");
        }}
      />
    </div>
  );
}

function ContextRow({
  ctx,
  rules,
  onEdit,
}: {
  ctx: Context;
  rules: Rule[];
  onEdit: () => void;
}) {
  const swatch = ctx.slug ? contextColorVar(ctx.slug) : ctx.color;
  const ctxRules = rules.filter((r) => r.context_id === ctx.id);
  return (
    <div className="flex items-center gap-[13px] px-4 py-[13px]">
      <span
        className="h-[18px] w-[18px] flex-shrink-0 border border-edge"
        style={{ background: swatch }}
      />
      <span className="w-[108px] flex-shrink-0 text-sm font-semibold uppercase tracking-[0.02em]">
        {ctx.name}
      </span>
      <span className="flex flex-1 flex-wrap items-center gap-1.5 text-[12.5px] text-muted">
        {ctxRules.length === 0 ? (
          <span className="italic">No rules yet</span>
        ) : (
          ctxRules.map((r) => (
            <code
              key={r.id}
              className="border border-rule bg-surface px-[7px] py-px font-sans font-semibold text-fg"
            >
              {r.match_field === "title" ? `title: ${r.pattern}` : r.pattern}
            </code>
          ))
        )}
      </span>
      <IconButton aria-label={`Edit ${ctx.name}`} onClick={onEdit}>
        ✎
      </IconButton>
    </div>
  );
}

function SettingsSkeleton() {
  return (
    <div className="flex flex-col gap-[18px]">
      {[0, 1, 2].map((i) => (
        <div key={i} className="border-[3px] border-edge">
          <Skeleton className="h-9 w-full" />
          <div className="flex flex-col gap-3 p-4">
            <Skeleton className="h-4 w-2/3" />
            <Skeleton className="h-4 w-1/2" />
          </div>
        </div>
      ))}
    </div>
  );
}
