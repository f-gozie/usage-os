import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";

import { Modal } from "@/components/ui/Modal";
import { Wordmark } from "@/components/ui/Wordmark";

const LINKS: ReadonlyArray<{ label: string; url: string }> = [
  { label: "Website", url: "https://usageos.app" },
  { label: "GitHub", url: "https://github.com/f-gozie/usage-os" },
  { label: "License · MIT", url: "https://github.com/f-gozie/usage-os/blob/main/LICENSE" },
  { label: "Sponsor", url: "https://github.com/sponsors/f-gozie" },
];

/**
 * Settings → About: a quiet brand moment. The dial-O wordmark, the version, the privacy promise,
 * and the links that matter. Links open in the system browser — user-initiated, so the app itself
 * still makes no network call (hard rule 1).
 */
export function AboutModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  // The running binary is the source of truth (a hardcoded string shipped 0.1.1 saying "0.1.0").
  const [version, setVersion] = useState("");
  useEffect(() => {
    void getVersion().then(setVersion).catch(() => undefined);
  }, []);

  return (
    <Modal open={open} onClose={onClose} title="About">
      <div className="flex flex-col items-center gap-3 py-2 text-center">
        <div className="flex justify-center">
          <Wordmark className="text-[44px]" />
        </div>
        {version && (
          <div className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted">
            Version {version}
          </div>
        )}
        <p className="max-w-[34ch] text-[14.5px] font-medium leading-relaxed">
          A calm, private look at where your time goes.{" "}
          <span className="font-semibold">Everything stays on your machine.</span>
        </p>
        <div className="mt-1 flex flex-wrap justify-center gap-2">
          {LINKS.map((l) => (
            <button
              key={l.label}
              type="button"
              onClick={() => void openUrl(l.url).catch(() => undefined)}
              className="border-2 border-edge px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.04em] transition-colors hover:border-c-deep hover:bg-c-deep hover:text-white"
            >
              {l.label}
            </button>
          ))}
        </div>
        <div className="mt-3 w-full border-t border-rule pt-3 text-[11.5px] text-muted">
          Open source under MIT · made by Favour · © 2026
        </div>
      </div>
    </Modal>
  );
}
