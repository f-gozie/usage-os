import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { Button } from "./Button";
import { Modal } from "./Modal";
import { TextInput } from "./TextInput";

const meta = {
  title: "UI/Modal",
  component: Modal,
} satisfies Meta<typeof Modal>;

export default meta;

export const AddExclusion: StoryObj = {
  render: () => {
    const [open, setOpen] = useState(true);
    return (
      <>
        <Button onClick={() => setOpen(true)}>Open</Button>
        <Modal
          open={open}
          onClose={() => setOpen(false)}
          title="Add exclusion"
          footer={
            <>
              <Button variant="secondary" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button onClick={() => setOpen(false)}>Add exclusion</Button>
            </>
          }
        >
          <TextInput label="Pattern" defaultValue="1Password" />
        </Modal>
      </>
    );
  },
};

export const DangerConfirm: StoryObj = {
  render: () => {
    const [open, setOpen] = useState(true);
    return (
      <>
        <Button variant="danger" onClick={() => setOpen(true)}>
          Delete
        </Button>
        <Modal
          open={open}
          onClose={() => setOpen(false)}
          title="Delete all data"
          danger
          className="max-w-[400px]"
          footer={
            <>
              <Button variant="secondary" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button variant="danger" disabled>
                Delete everything
              </Button>
            </>
          }
        >
          <p className="text-[12.5px] font-semibold leading-relaxed text-c-research">
            This wipes every recorded event. It cannot be undone.
          </p>
          <TextInput label="Type DELETE to confirm" placeholder="DELETE" />
        </Modal>
      </>
    );
  },
};
