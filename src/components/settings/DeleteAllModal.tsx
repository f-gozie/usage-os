import { useEffect, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { TextInput } from "@/components/ui/TextInput";
import { deleteAllData } from "@/lib/tauri";

const CONFIRM_WORD = "DELETE";

export interface DeleteAllModalProps {
  open: boolean;
  onClose: () => void;
  /** Called after a successful wipe (refresh + notice). */
  onDeleted: () => void | Promise<void>;
}

export function DeleteAllModal({ open, onClose, onDeleted }: DeleteAllModalProps) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setText("");
    setError(null);
  }, [open]);

  const confirm = async () => {
    setBusy(true);
    setError(null);
    try {
      await deleteAllData();
      await onDeleted();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't delete the data.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Delete all data"
      danger
      className="max-w-[400px]"
      footer={
        <>
          <Button variant="secondary" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button variant="danger" onClick={confirm} disabled={busy || text !== CONFIRM_WORD}>
            Delete everything
          </Button>
        </>
      }
    >
      <p className="text-[12.5px] font-semibold leading-relaxed text-c-research">
        This wipes every recorded event from{" "}
        <code className="font-sans">usage.db</code>. Your contexts, rules and exclusions stay. It
        cannot be undone.
      </p>
      <TextInput
        label={`Type ${CONFIRM_WORD} to confirm`}
        value={text}
        onChange={(e) => setText(e.target.value)}
        autoFocus
        placeholder={CONFIRM_WORD}
      />
      {error && <p className="text-[11px] font-semibold text-c-research">{error}</p>}
    </Modal>
  );
}
