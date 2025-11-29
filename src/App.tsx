import { useState, useEffect } from 'react';
import { StatsCard } from '@/components/StatsCard';
import { ActivityChart } from '@/components/ActivityChart';
import { TimeRangeSelector, TimeRange } from '@/components/TimeRangeSelector';
import { StatsCardSkeleton, ChartSkeleton } from '@/components/SkeletonLoader';
import { SettingsView } from '@/components/SettingsView';
import { getActivityStats, ActivityLog, getCategories, Category } from '@/lib/tauri';
import { 
  calculateDuration, 
  calculateIdleDuration,
  groupByProcess, 
  groupByCategory,
  formatDuration, 
  getTodayRange, 
  getYesterdayRange, 
  getWeekRange 
} from '@/lib/stats';
import { formatRelativeTime } from '@/lib/time';
import { RotateCw, Settings, LayoutDashboard } from 'lucide-react';

function App() {
  const [currentView, setCurrentView] = useState<'dashboard' | 'settings'>('dashboard');
  const [timeRange, setTimeRange] = useState<TimeRange>('today');
  const [todayData, setTodayData] = useState<ActivityLog[]>([]);
  const [yesterdayData, setYesterdayData] = useState<ActivityLog[]>([]);
  const [displayData, setDisplayData] = useState<ActivityLog[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastRefreshed, setLastRefreshed] = useState<Date>(new Date());
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [relativeTime, setRelativeTime] = useState('');
  const [showIdle, setShowIdle] = useState(true);
  const [groupBy, setGroupBy] = useState<'process' | 'category'>('process');

  const fetchData = async () => {
    try {
      setLoading(true);
      setError(null);

      const [todayStart, todayEnd] = getTodayRange();
      const [yesterdayStart, yesterdayEnd] = getYesterdayRange();

      const [todayLogs, yesterdayLogs, cats] = await Promise.all([
        getActivityStats(todayStart, todayEnd),
        getActivityStats(yesterdayStart, yesterdayEnd),
        getCategories(),
      ]);

      setTodayData(todayLogs);
      setYesterdayData(yesterdayLogs);
      setCategories(cats);

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

  const processStats = groupByProcess(displayData, categories, showIdle);
  const categoryStats = groupByCategory(displayData, categories, showIdle);

  const chartData = groupBy === 'process' 
    ? processStats.map(stat => ({
        name: stat.processName,
        displayName: stat.displayName,
        totalDuration: stat.totalDuration,
        percentage: stat.percentage,
        isIdle: stat.isIdle,
        // Don't use category color for process view to ensure distinctness
        color: undefined 
    }))
    : categoryStats.map(stat => ({
        name: stat.categoryName,
        displayName: stat.categoryName,
        totalDuration: stat.totalDuration,
        percentage: stat.percentage,
        isIdle: false,
        color: stat.categoryColor
    }));

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
      <div className="absolute top-0 left-0 w-16 h-16 border-l-2 border-t-2 border-neon-cyan/20 pointer-events-none" />
      <div className="absolute top-0 right-0 w-16 h-16 border-r-2 border-t-2 border-neon-purple/20 pointer-events-none" />
      <div className="absolute bottom-0 left-0 w-16 h-16 border-l-2 border-b-2 border-neon-green/20 pointer-events-none" />
      <div className="absolute bottom-0 right-0 w-16 h-16 border-r-2 border-b-2 border-neon-cyan/20 pointer-events-none" />

      <div className="container mx-auto p-6 flex flex-col h-screen max-w-6xl overflow-hidden">
        {/* Header */}
        <header className="flex flex-wrap items-start justify-between gap-4 shrink-0 mb-6">
          <div className="space-y-1">
            <h1 className="usage-logo font-mono text-3xl">
              USAGE<span className="usage-logo__glyph">▧</span>OS
            </h1>
            <p className="text-xs text-muted-foreground tracking-[0.3em] uppercase">
              Activity Tracking Dashboard
            </p>
          </div>
          
          <div className="flex flex-col items-end gap-2">
            <div className="flex items-center gap-2 bg-card/30 rounded-lg p-1 border border-border/50">
              <button
                onClick={() => setCurrentView('dashboard')}
                className={`p-1.5 rounded-md transition-all ${
                  currentView === 'dashboard' 
                    ? 'bg-neon-cyan/20 text-neon-cyan shadow-[0_0_10px_rgba(0,255,255,0.2)]' 
                    : 'text-muted-foreground hover:text-foreground'
                }`}
                title="Dashboard"
              >
                <LayoutDashboard className="w-4 h-4" />
              </button>
              <button
                onClick={() => setCurrentView('settings')}
                className={`p-1.5 rounded-md transition-all ${
                  currentView === 'settings' 
                    ? 'bg-neon-cyan/20 text-neon-cyan shadow-[0_0_10px_rgba(0,255,255,0.2)]' 
                    : 'text-muted-foreground hover:text-foreground'
                }`}
                title="Settings"
              >
                <Settings className="w-4 h-4" />
              </button>
            </div>

            {currentView === 'dashboard' && (
              <div className="flex items-center gap-2">
                <button
                  onClick={handleRefresh}
                  disabled={isRefreshing || loading}
                  className="flex items-center gap-2 px-2 py-1 rounded-md text-xs border border-border bg-card/40 text-muted-foreground hover:text-foreground hover:bg-card/60 transition-all disabled:opacity-50"
                >
                  <RotateCw className={`w-3 h-3 ${isRefreshing ? 'animate-spin' : ''}`} />
                  <span className="font-mono uppercase">Refresh</span>
                </button>
                <p className="text-[10px] text-muted-foreground font-mono tracking-widest">
                  {relativeTime}
                </p>
              </div>
            )}
          </div>
        </header>

        {currentView === 'settings' ? (
          <SettingsView />
        ) : (
          <div className="flex-1 min-h-0 flex flex-col gap-4 animate-in fade-in slide-in-from-bottom-4 duration-500 pb-2 overflow-y-auto custom-scrollbar">
            {/* Stats Cards Grid */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 shrink-0">
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
            <div className="shrink-0">
              <TimeRangeSelector value={timeRange} onChange={setTimeRange} />
            </div>

            {/* Activity Chart */}
            <div className="flex-1 min-h-[400px] flex flex-col space-y-3">
              <div className="flex flex-wrap items-center justify-between gap-3 shrink-0">
                <div className="flex flex-col">
                  <span className="text-xs uppercase tracking-[0.3em] text-muted-foreground">
                    Activity Distribution
                  </span>
                  <div className="flex gap-2 mt-1">
                     <button
                        onClick={() => setGroupBy('process')}
                        className={`text-xs font-mono transition-colors ${groupBy === 'process' ? 'text-neon-cyan underline decoration-neon-cyan/50' : 'text-muted-foreground hover:text-foreground'}`}
                     >
                        By Process
                     </button>
                     <span className="text-xs text-muted-foreground">|</span>
                     <button
                        onClick={() => setGroupBy('category')}
                        className={`text-xs font-mono transition-colors ${groupBy === 'category' ? 'text-neon-cyan underline decoration-neon-cyan/50' : 'text-muted-foreground hover:text-foreground'}`}
                     >
                        By Category
                     </button>
                  </div>
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

              <div className="flex-1 min-h-0">
                {loading ? (
                  <ChartSkeleton />
                ) : (
                  <ActivityChart data={chartData} />
                )}
              </div>
            </div>
          </div>
        )}

        {/* Footer */}
        <footer className="text-center text-xs text-muted-foreground font-mono py-2 shrink-0">
          Data refreshes every 30 seconds • 100% local storage
        </footer>
      </div>
    </div>
  );
}

export default App;
