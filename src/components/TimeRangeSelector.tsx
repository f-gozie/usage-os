// import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';

export type TimeRange = 'today' | 'week';

interface TimeRangeSelectorProps {
  value: TimeRange;
  onChange: (value: TimeRange) => void;
}

export function TimeRangeSelector({ value, onChange }: TimeRangeSelectorProps) {
  return (
    <div className="flex p-1 bg-card/30 rounded-lg border border-border/50 w-fit">
      <button
        onClick={() => onChange('today')}
        className={`px-4 py-1.5 rounded-md text-xs font-mono transition-all ${
          value === 'today'
            ? 'bg-neon-cyan/10 text-neon-cyan shadow-[0_0_10px_rgba(0,255,255,0.1)]'
            : 'text-muted-foreground hover:text-foreground'
        }`}
      >
        TODAY
      </button>
      <button
        onClick={() => onChange('week')}
        className={`px-4 py-1.5 rounded-md text-xs font-mono transition-all ${
          value === 'week'
            ? 'bg-neon-cyan/10 text-neon-cyan shadow-[0_0_10px_rgba(0,255,255,0.1)]'
            : 'text-muted-foreground hover:text-foreground'
        }`}
      >
        PAST WEEK
      </button>
    </div>
  );
}

