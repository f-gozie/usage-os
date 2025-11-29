import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts';
import { Card, CardContent } from '@/components/ui/card';
import { ProcessStats, formatDuration, getColorForProcess } from '@/lib/stats';

interface ActivityChartProps {
  data: ProcessStats[];
}

export function ActivityChart({ data }: ActivityChartProps) {
  const top5 = data.slice(0, 5);
  const others = data.slice(5);
  
  const chartData = top5.map((stat, index) => ({
    name: stat.processName,
    label: stat.displayName,
    value: stat.totalDuration,
    percentage: stat.percentage,
    isIdle: stat.isIdle,
    color: stat.isIdle
      ? 'hsl(240, 3%, 35%)'
      : getColorForProcess(stat.processName, index),
  }));

  if (others.length > 0) {
    const otherTotal = others.reduce((sum, stat) => sum + stat.totalDuration, 0);
    const otherPercentage = others.reduce((sum, stat) => sum + stat.percentage, 0);
    chartData.push({
      name: 'Other',
      label: 'Other',
      value: otherTotal,
      percentage: otherPercentage,
      isIdle: false,
      color: 'hsl(240, 5%, 45%)',
    });
  }

  if (chartData.length === 0) {
    return (
      <Card className="border-border/50">
        <CardContent className="flex flex-col items-center justify-center h-64 text-center space-y-4">
          <div className="w-16 h-16 rounded-full border-2 border-neon-cyan/30 flex items-center justify-center">
            <svg
              className="w-8 h-8 text-neon-cyan"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"
              />
            </svg>
          </div>
          <div className="space-y-2">
            <p className="text-sm font-medium text-foreground">No activity data yet</p>
            <p className="text-xs text-muted-foreground max-w-xs">
              Start using your computer to see activity data.<br />
              Data appears within 5 seconds.
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="border-border/50 hover:border-neon-purple/30 transition-all duration-200 hover:-translate-y-0.5 relative">
      <div className="absolute top-0 right-0 w-1 h-full bg-gradient-to-b from-neon-purple/50 to-transparent" />
      
      <CardContent>
        <ResponsiveContainer width="100%" height={320}>
          <PieChart>
            <Pie
              data={chartData}
              cx="50%"
              cy="50%"
              innerRadius={70}
              outerRadius={120}
              paddingAngle={2}
              dataKey="value"
            >
              {chartData.map((entry, index) => (
                <Cell 
                  key={`cell-${index}`} 
                  fill={entry.color}
                  stroke={entry.color}
                  strokeWidth={entry.isIdle ? 1 : 2}
                  strokeDasharray={entry.isIdle ? '4 4' : undefined}
                  className="hover:opacity-80 transition-opacity"
                />
              ))}
            </Pie>
            <Tooltip
              content={({ active, payload }) => {
                if (active && payload && payload.length) {
                  const data = payload[0].payload;
                  return (
                    <div className="bg-card/95 backdrop-blur-sm border border-border/50 rounded-lg p-3 shadow-lg">
                      <p className="font-medium text-sm mb-1">{data.label}</p>
                      <p className="font-mono text-xs text-neon-cyan">
                        {formatDuration(data.value)}
                      </p>
                      <p className="font-mono text-xs text-muted-foreground">
                        {data.percentage.toFixed(1)}%
                      </p>
                    </div>
                  );
                }
                return null;
              }}
            />
          </PieChart>
        </ResponsiveContainer>
        
        <div className="mt-4 space-y-2">
          {chartData.map((entry, index) => (
            <div key={index} className="flex items-center justify-between text-sm">
              <div className="flex items-center gap-2">
                <div 
                  className={`w-3 h-3 rounded-sm ${entry.isIdle ? 'border border-border' : ''}`} 
                  style={{ backgroundColor: entry.color }}
                />
                <span className="text-foreground truncate max-w-[200px]">
                  {entry.label}
                </span>
              </div>
              <div className="flex items-center gap-3">
                <span className="font-mono text-xs text-muted-foreground">
                  {entry.percentage.toFixed(1)}%
                </span>
                <span className="font-mono text-xs text-foreground min-w-[60px] text-right">
                  {formatDuration(entry.value)}
                </span>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

