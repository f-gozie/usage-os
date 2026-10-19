export interface RecapCardProps {
  text: string;
}

/** The daily recap card: the shape of the day, written by the app from the facts (D74). */
export function RecapCard({ text }: RecapCardProps) {
  return (
    <div className="border-[3px] border-edge bg-surface px-[18px] py-4">
      <div className="mb-2.5 text-[10.5px] font-semibold uppercase tracking-[0.16em] text-muted">
        Daily recap
      </div>
      <p className="text-[17px] font-medium leading-[1.45]">{text}</p>
    </div>
  );
}
