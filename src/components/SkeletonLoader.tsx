import { cn } from '@/lib/utils';

interface SkeletonProps {
  className?: string;
}

export function Skeleton({ className }: SkeletonProps) {
  return (
    <div
      className={cn(
        'animate-pulse rounded-md bg-muted/50',
        className
      )}
    />
  );
}

export function StatsCardSkeleton() {
  return (
    <div className="relative overflow-hidden border border-border/50 rounded-lg p-6">
      <div className="absolute top-0 left-0 w-1 h-full bg-gradient-to-b from-neon-cyan/50 to-transparent" />
      
      <div className="space-y-4">
        <Skeleton className="h-3 w-20" />
        <Skeleton className="h-8 w-32" />
        <Skeleton className="h-4 w-24" />
      </div>
    </div>
  );
}

export function ChartSkeleton() {
  return (
    <div className="border border-border/50 rounded-lg p-6">
      <div className="space-y-4">
        <Skeleton className="h-4 w-48" />
        
        <div className="flex items-center justify-center h-[300px]">
          <div className="relative w-48 h-48">
            <Skeleton className="w-full h-full rounded-full" />
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="w-28 h-28 rounded-full bg-background" />
            </div>
          </div>
        </div>
        
        <div className="space-y-2">
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-full" />
        </div>
      </div>
    </div>
  );
}

