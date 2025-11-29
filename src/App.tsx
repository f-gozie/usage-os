import { useState, useEffect } from 'react';
import { StatsCard } from '@/components/StatsCard';
import { ActivityChart } from '@/components/ActivityChart';
import { TimeRangeSelector, TimeRange } from '@/components/TimeRangeSelector';
import { StatsCardSkeleton, ChartSkeleton } from '@/components/SkeletonLoader';
import { getActivityStats, ActivityLog } from '@/lib/tauri';
import { 
  calculateDuration, 
  calculateIdleDuration,
  groupByProcess, 
  formatDuration, 
  getTodayRange, 
  getYesterdayRange, 
  getWeekRange 
} from '@/lib/stats';
import { formatRelativeTime } from '@/lib/time';
import { RotateCw } from 'lucide-react';

function App() {
  const [timeRange, setTimeRange] = useState<TimeRange>('today');
  const [todayData, setTodayData] = useState<ActivityLog[]>([]);
  const [yesterdayData, setYesterdayData] = useState<ActivityLog[]>([]);
  const [displayData, setDisplayData] = useState<ActivityLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastRefreshed, setLastRefreshed] = useState<Date>(new Date());
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [relativeTime, setRelativeTime] = useState('');
  const [showIdle, setShowIdle] = useState(true);

  const fetchData = async () => {
    try {
      setLoading(true);
      setError(null);

      const [todayStart, todayEnd] = getTodayRange();
      const [yesterdayStart, yesterdayEnd] = getYesterdayRange();

      const [todayLogs, yesterdayLogs] = await Promise.all([
        getActivityStats(todayStart, todayEnd),
        getActivityStats(yesterdayStart, yesterdayEnd),
      ]);

      setTodayData(todayLogs);
      setYesterdayData(yesterdayLogs);

      if (timeRange === 'today') {
        setDisplayData(todayLogs);
      } else {
        const [weekStart, weekEnd] = getWeekRange();
        const weekLogs = await getActivityStats(weekStart, weekEnd);
        setDisplayData(weekLogs);
      }
      
      setLastRefreshed(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch activity data');
      console.error('Error fetching data:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleRefresh = async () => {
    setIsRefreshing(true);
    await fetchData();
    setIsRefreshing(false);
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, [timeRange]);

  useEffect(() => {
    const updateRelativeTime = () => {
      setRelativeTime(formatRelativeTime(lastRefreshed));
    };
    
    updateRelativeTime();
    const interval = setInterval(updateRelativeTime, 1000);
    return () => clearInterval(interval);
  }, [lastRefreshed]);

  const todayDuration = calculateDuration(todayData);
  const yesterdayDuration = calculateDuration(yesterdayData);
  const percentageChange = yesterdayDuration > 0 
    ? ((todayDuration - yesterdayDuration) / yesterdayDuration) * 100 
    : 0;

  const todayIdleDuration = calculateIdleDuration(todayData);
  const yesterdayIdleDuration = calculateIdleDuration(yesterdayData);
  const idleChange = yesterdayIdleDuration > 0
    ? ((todayIdleDuration - yesterdayIdleDuration) / yesterdayIdleDuration) * 100
    : 0;
  const totalTrackedToday = calculateDuration(todayData, { includeIdle: true });
  const idlePercentOfDay = totalTrackedToday > 0
    ? (todayIdleDuration / totalTrackedToday) * 100
    : 0;

  const processStats = groupByProcess(displayData, showIdle);

  if (error) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center p-4">
        <div className="text-center space-y-4">
          <div className="text-red-400 font-mono text-sm">{error}</div>
          <button
            onClick={fetchData}
            className="px-4 py-2 bg-neon-cyan/10 border border-neon-cyan/30 text-neon-cyan rounded-lg hover:bg-neon-cyan/20 transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background text-foreground relative scanline">
      {/* Corner decorations */}
      <div className="absolute top-0 left-0 w-16 h-16 border-l-2 border-t-2 border-neon-cyan/20" />
      <div className="absolute top-0 right-0 w-16 h-16 border-r-2 border-t-2 border-neon-purple/20" />
      <div className="absolute bottom-0 left-0 w-16 h-16 border-l-2 border-b-2 border-neon-green/20" />
      <div className="absolute bottom-0 right-0 w-16 h-16 border-r-2 border-b-2 border-neon-cyan/20" />

      <div className="container mx-auto p-6 space-y-6 max-w-6xl">
        {/* Header */}
        <header className="flex flex-wrap items-start justify-between gap-4">
          <div className="space-y-1">
            <h1 className="usage-logo font-mono text-3xl">
              USAGE<span className="usage-logo__glyph">▧</span>OS
            </h1>
            <p className="text-xs text-muted-foreground tracking-[0.3em] uppercase">
              Activity Tracking Dashboard
            </p>
          </div>
          
          <div className="flex flex-col items-end gap-1">
            <button
              onClick={handleRefresh}
              disabled={isRefreshing || loading}
              className="flex items-center gap-2 px-3 py-2 rounded-lg border border-border bg-card/40 text-foreground hover:bg-card/60 hover:glow-cyan transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <RotateCw className={`w-4 h-4 ${isRefreshing ? 'animate-spin' : ''}`} />
              <span className="text-xs font-mono uppercase">Refresh</span>
            </button>
            <p className="text-[11px] text-muted-foreground font-mono tracking-widest">
              {relativeTime}
            </p>
          </div>
        </header>

        {/* Stats Cards Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {loading ? (
            <>
              <StatsCardSkeleton />
              <StatsCardSkeleton />
              <StatsCardSkeleton />
            </>
          ) : (
            <>
              <StatsCard
                title="Today"
                value={formatDuration(todayDuration)}
                comparison={{
                  value: 'yesterday',
                  percentage: percentageChange,
                }}
              />
              <StatsCard
                title="Yesterday"
                value={formatDuration(yesterdayDuration)}
              />
              <StatsCard
                title="Idle Time"
                subtitle={`${idlePercentOfDay.toFixed(1)}% of tracked time`}
                value={formatDuration(todayIdleDuration)}
                comparison={{
                  value: 'yesterday',
                  percentage: idleChange,
                }}
                className="border-neon-purple/30 hover:border-neon-purple/40"
              />
            </>
          )}
        </div>

        {/* Time Range Selector */}
        <TimeRangeSelector value={timeRange} onChange={setTimeRange} />

        {/* Activity Chart */}
        <div className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-col">
              <span className="text-xs uppercase tracking-[0.3em] text-muted-foreground">
                Activity Distribution
              </span>
              <span className="text-sm text-muted-foreground/80 font-mono">
                {timeRange === 'today' ? 'Today' : 'Past Week'}
              </span>
            </div>
            <button
              onClick={() => setShowIdle((prev) => !prev)}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-full border text-xs font-mono transition-all ${
                showIdle
                  ? 'border-neon-cyan/60 text-neon-cyan bg-neon-cyan/10'
                  : 'border-border text-muted-foreground'
              }`}
            >
              <span className="uppercase tracking-widest">Idle</span>
              <span>{showIdle ? 'ON' : 'OFF'}</span>
            </button>
          </div>

          {loading ? (
            <ChartSkeleton />
          ) : (
            <ActivityChart data={processStats} />
          )}
        </div>

        {/* Footer */}
        <footer className="text-center text-xs text-muted-foreground font-mono pt-4">
          Data refreshes every 30 seconds • 100% local storage
        </footer>
      </div>
    </div>
  );
}

export default App;
