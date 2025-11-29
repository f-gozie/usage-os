import { Card, CardContent, CardDescription, CardHeader } from '@/components/ui/card';
import { ArrowUp, ArrowDown, Minus } from 'lucide-react';
import { cn } from '@/lib/utils';

interface StatsCardProps {
  title: string;
  value: string;
  subtitle?: string;
  comparison?: {
    value: string;
    percentage: number;
  };
  className?: string;
}

export function StatsCard({ title, value, subtitle, comparison, className }: StatsCardProps) {
  const getComparisonIcon = () => {
    if (!comparison) return null;
    
    if (comparison.percentage > 0) {
      return <ArrowUp className="w-4 h-4 text-neon-green" />;
    } else if (comparison.percentage < 0) {
      return <ArrowDown className="w-4 h-4 text-red-400" />;
    }
    return <Minus className="w-4 h-4 text-muted-foreground" />;
  };

  const getComparisonColor = () => {
    if (!comparison) return '';
    
    if (comparison.percentage > 0) return 'text-neon-green';
    if (comparison.percentage < 0) return 'text-red-400';
    return 'text-muted-foreground';
  };

  return (
    <Card className={cn(
      'relative overflow-hidden border-border/50 hover:border-neon-cyan/30 transition-all duration-200 hover:glow-cyan hover:-translate-y-0.5 active:scale-[0.98]',
      className
    )}>
      <div className="absolute top-0 left-0 w-1 h-full bg-gradient-to-b from-neon-cyan/50 to-transparent" />
      
      <CardHeader className="pb-2 space-y-1">
        <CardDescription className="text-xs uppercase tracking-wider text-muted-foreground">
          {title}
        </CardDescription>
        {subtitle && (
          <span className="text-[10px] uppercase tracking-widest text-muted-foreground/80">
            {subtitle}
          </span>
        )}
      </CardHeader>
      
      <CardContent>
        <div className="space-y-1">
          <div className="text-3xl font-mono font-bold tabular-nums tracking-tight fade-in">
            {value}
          </div>
          
          {comparison && (
            <div className="flex items-center gap-1 text-sm">
              {getComparisonIcon()}
              <span className={cn('font-mono text-xs', getComparisonColor())}>
                {Math.abs(comparison.percentage).toFixed(1)}%
              </span>
              <span className="text-xs text-muted-foreground ml-1">
                vs {comparison.value}
              </span>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

