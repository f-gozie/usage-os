import { useState, useEffect } from 'react';
import { 
  Category, 
  Rule, 
  getCategories, 
  createCategory, 
  deleteCategory, 
  getRules, 
  createRule, 
  deleteRule, 
  reprocessLogs,
  getSettings,
  updateSetting,
} from '@/lib/tauri';
import { Trash2, Plus, RefreshCw, Layers, ListFilter, CheckCircle, AlertCircle, Database } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export function SettingsView() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [rules, setRules] = useState<Rule[]>([]);
  const [loading, setLoading] = useState(true);
  
  const [newCatName, setNewCatName] = useState('');
  const [newCatColor, setNewCatColor] = useState('#00ffff');
  
  const [newRuleCatId, setNewRuleCatId] = useState<string>('');
  const [newRuleMatchField, setNewRuleMatchField] = useState('process');
  const [newRulePattern, setNewRulePattern] = useState('');
  
  const [retentionDays, setRetentionDays] = useState('0');
  const [reprocessing, setReprocessing] = useState(false);
  const [feedback, setFeedback] = useState<{ type: 'success' | 'error', message: string } | null>(null);

  useEffect(() => {
    if (feedback) {
      const timer = setTimeout(() => setFeedback(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [feedback]);

  useEffect(() => {
    fetchData();
  }, []);

  useEffect(() => {
    if (categories.length > 0 && !newRuleCatId) {
      setNewRuleCatId(categories[0].id.toString());
    }
  }, [categories]);

  const fetchData = async () => {
    setLoading(true);
    try {
      const [cats, rls, settings] = await Promise.all([getCategories(), getRules(), getSettings()]);
      setCategories(cats);
      setRules(rls);
      const retSetting = settings.find(([k]: [string, string]) => k === 'data_retention_days');
      if (retSetting) setRetentionDays(retSetting[1]);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const handleRetentionChange = async (value: string) => {
    setRetentionDays(value);
    try {
      await updateSetting('data_retention_days', value);
      setFeedback({ type: 'success', message: `Data retention updated to ${value === '0' ? 'Keep All' : value + ' days'}` });
    } catch (e) {
      console.error(e);
      setFeedback({ type: 'error', message: 'Failed to update retention setting.' });
    }
  };

  const handleCreateCategory = async () => {
    if (!newCatName) return;
    try {
      await createCategory(newCatName, newCatColor);
      setNewCatName('');
      fetchData();
    } catch (e) {
      console.error(e);
      if (typeof e === 'string' && e.includes('UNIQUE constraint failed')) {
        setFeedback({ type: 'error', message: 'Category name must be unique.' });
      } else {
        setFeedback({ type: 'error', message: 'Failed to create category.' });
      }
    }
  };

  const handleDeleteCategory = async (id: number) => {
    try {
      await deleteCategory(id);
      fetchData();
    } catch (e) {
      console.error(e);
    }
  };

  const handleCreateRule = async () => {
    if (!newRuleCatId || !newRulePattern) return;
    try {
      await createRule(parseInt(newRuleCatId), newRuleMatchField, newRulePattern);
      setNewRulePattern('');
      fetchData();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeleteRule = async (id: number) => {
    try {
      await deleteRule(id);
      fetchData();
    } catch (e) {
      console.error(e);
    }
  };

  const handleReprocess = async () => {
    setReprocessing(true);
    setFeedback(null);
    try {
      await reprocessLogs();
      setFeedback({ type: 'success', message: 'Logs successfully reprocessed!' });
    } catch (e) {
      console.error(e);
      setFeedback({ type: 'error', message: 'Failed to reprocess logs.' });
    } finally {
      setReprocessing(false);
    }
  };

  if (loading && categories.length === 0) {
    return <div className="p-4 text-center font-mono text-neon-cyan animate-pulse">Loading configuration...</div>;
  }

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 flex-1 flex flex-col h-full">
      
      <div className="flex justify-between items-center shrink-0">
        <div>
          <h2 className="text-xl font-mono text-foreground">Configuration</h2>
          <p className="text-xs text-muted-foreground tracking-widest uppercase">Manage Categories & Rules</p>
        </div>
        <div className="flex items-center gap-4">
            {feedback && (
                <div className={`flex items-center gap-2 text-xs font-mono ${feedback.type === 'success' ? 'text-neon-green' : 'text-red-400'} animate-in fade-in slide-in-from-right-4`}>
                    {feedback.type === 'success' ? <CheckCircle className="w-3 h-3" /> : <AlertCircle className="w-3 h-3" />}
                    {feedback.message}
                </div>
            )}
            <button
            onClick={handleReprocess}
            disabled={reprocessing}
            className="flex items-center gap-2 px-4 py-2 bg-neon-purple/10 border border-neon-purple/50 text-neon-purple rounded-lg hover:bg-neon-purple/20 transition-all disabled:opacity-50"
            >
            <RefreshCw className={`w-4 h-4 ${reprocessing ? 'animate-spin' : ''}`} />
            <span className="text-xs font-mono uppercase">Reprocess Logs</span>
            </button>
        </div>
      </div>

      {/* Data Retention */}
      <Card className="border-border/50 bg-card/30 backdrop-blur shrink-0">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base font-mono uppercase tracking-widest text-neon-purple">
            <Database className="w-4 h-4" /> Data Retention
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-4">
            <span className="text-sm text-muted-foreground">Keep activity data for:</span>
            <select
              value={retentionDays}
              onChange={(e) => handleRetentionChange(e.target.value)}
              className="bg-background border border-border rounded px-3 py-2 text-sm focus:border-neon-purple focus:outline-none transition-colors"
            >
              <option value="0">Keep All</option>
              <option value="30">30 days</option>
              <option value="60">60 days</option>
              <option value="90">90 days</option>
              <option value="180">180 days</option>
              <option value="365">365 days</option>
            </select>
            <span className="text-xs text-muted-foreground">
              {retentionDays === '0' ? 'No automatic cleanup' : `Logs older than ${retentionDays} days are deleted on startup`}
            </span>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-6 md:grid-cols-2 flex-1 min-h-0">
        
        <Card className="border-border/50 bg-card/30 backdrop-blur flex flex-col h-full">
          <CardHeader className="shrink-0">
            <CardTitle className="flex items-center gap-2 text-base font-mono uppercase tracking-widest text-neon-cyan">
              <Layers className="w-4 h-4" /> Categories
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 flex-1 flex flex-col min-h-0">
            <div className="flex gap-2 items-center">
              <div className="shrink-0">
                <input
                  type="color"
                  value={newCatColor}
                  onChange={(e) => setNewCatColor(e.target.value)}
                  className="w-10 h-10 p-1 rounded bg-background border border-border cursor-pointer block"
                />
              </div>
              <div className="flex-1 min-w-0">
                <input
                  type="text"
                  value={newCatName}
                  onChange={(e) => setNewCatName(e.target.value)}
                  placeholder="Category Name"
                  className="w-full bg-background border border-border rounded px-3 py-2 text-sm focus:border-neon-cyan focus:outline-none transition-colors"
                  onKeyDown={(e) => e.key === 'Enter' && handleCreateCategory()}
                />
              </div>
              <button
                onClick={handleCreateCategory}
                disabled={!newCatName}
                className="shrink-0 p-2 bg-neon-cyan/10 border border-neon-cyan/50 text-neon-cyan rounded hover:bg-neon-cyan/20 disabled:opacity-50"
              >
                <Plus className="w-5 h-5" />
              </button>
            </div>

            <div className="space-y-2 overflow-y-auto pr-2 custom-scrollbar flex-1 min-h-0">
              {categories.map((cat) => (
                <div key={cat.id} className="flex items-center justify-between p-2 rounded bg-card/50 border border-border/50 group hover:border-border transition-colors shrink-0">
                  <div className="flex items-center gap-3">
                    <div 
                      className="w-3 h-3 rounded-full shadow-[0_0_8px_currentColor]" 
                      style={{ backgroundColor: cat.color, color: cat.color }}
                    />
                    <span className="text-sm font-medium">{cat.name}</span>
                  </div>
                  <button
                    onClick={() => handleDeleteCategory(cat.id)}
                    className="text-muted-foreground hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              ))}
              {categories.length === 0 && (
                <div className="text-xs text-muted-foreground text-center py-4 italic">
                  No categories defined.
                </div>
              )}
            </div>
          </CardContent>
        </Card>

        <Card className="border-border/50 bg-card/30 backdrop-blur flex flex-col h-full">
          <CardHeader className="shrink-0">
            <CardTitle className="flex items-center gap-2 text-base font-mono uppercase tracking-widest text-neon-green">
              <ListFilter className="w-4 h-4" /> Rules
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 flex-1 flex flex-col min-h-0">
            <div className="space-y-2 p-3 bg-background/50 rounded border border-border/50 shrink-0">
              <div className="flex gap-2 text-xs font-mono text-muted-foreground mb-1">
                IF
                <select
                  value={newRuleMatchField}
                  onChange={(e) => setNewRuleMatchField(e.target.value)}
                  className="bg-transparent border-b border-border text-foreground focus:outline-none focus:border-neon-green"
                >
                  <option value="process">Process Name</option>
                  <option value="title">Window Title</option>
                </select>
                CONTAINS
              </div>
              
              <input
                type="text"
                value={newRulePattern}
                onChange={(e) => setNewRulePattern(e.target.value)}
                placeholder="text to match..."
                className="w-full bg-background border border-border rounded px-3 py-1.5 text-sm focus:border-neon-green focus:outline-none transition-colors"
                onKeyDown={(e) => e.key === 'Enter' && handleCreateRule()}
              />
              
              <div className="flex gap-2 items-center text-xs font-mono text-muted-foreground mt-1">
                THEN SET CATEGORY TO
                <select
                  value={newRuleCatId}
                  onChange={(e) => setNewRuleCatId(e.target.value)}
                  className="flex-1 bg-transparent border-b border-border text-foreground focus:outline-none focus:border-neon-green max-w-[150px]"
                >
                  {categories.map((cat) => (
                    <option key={cat.id} value={cat.id}>
                      {cat.name}
                    </option>
                  ))}
                </select>
                <button
                  onClick={handleCreateRule}
                  disabled={!newRulePattern || !newRuleCatId}
                  className="ml-auto p-1.5 bg-neon-green/10 border border-neon-green/50 text-neon-green rounded hover:bg-neon-green/20 disabled:opacity-50"
                >
                  <Plus className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="space-y-2 overflow-y-auto pr-2 custom-scrollbar flex-1 min-h-0">
              {rules.map((rule) => {
                const category = categories.find(c => c.id === rule.category_id);
                return (
                  <div key={rule.id} className="flex items-center justify-between p-2 rounded bg-card/50 border border-border/50 group hover:border-border transition-colors text-xs shrink-0">
                    <div className="flex-1 grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5">
                      <span className="text-muted-foreground font-mono">Match:</span>
                      <span className="font-mono text-foreground truncate">
                        {rule.match_field === 'process' ? 'Process' : 'Title'} contains "{rule.pattern}"
                      </span>
                      
                      <span className="text-muted-foreground font-mono">Set:</span>
                      <span 
                        className="font-medium flex items-center gap-1.5" 
                        style={{ color: category?.color || 'inherit' }}
                      >
                         {category?.name || 'Unknown'}
                      </span>
                    </div>
                    <button
                      onClick={() => handleDeleteRule(rule.id)}
                      className="ml-2 text-muted-foreground hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                );
              })}
              {rules.length === 0 && (
                <div className="text-xs text-muted-foreground text-center py-4 italic">
                  No rules defined.
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

