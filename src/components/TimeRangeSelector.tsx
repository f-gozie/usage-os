import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';

export type TimeRange = 'today' | 'week';

interface TimeRangeSelectorProps {
  value: TimeRange;
  onChange: (value: TimeRange) => void;
}

export function TimeRangeSelector({ value, onChange }: TimeRangeSelectorProps) {
  return (
    <Tabs value={value} onValueChange={(v) => onChange(v as TimeRange)}>
      <TabsList className="grid w-full grid-cols-2 bg-card border border-border/50">
        <TabsTrigger 
          value="today"
          className="data-[state=active]:bg-neon-cyan/10 data-[state=active]:text-neon-cyan data-[state=active]:border-neon-cyan/50 transition-all duration-200"
        >
          Today
        </TabsTrigger>
        <TabsTrigger 
          value="week"
          className="data-[state=active]:bg-neon-purple/10 data-[state=active]:text-neon-purple data-[state=active]:border-neon-purple/50 transition-all duration-200"
        >
          Past Week
        </TabsTrigger>
      </TabsList>
    </Tabs>
  );
}

