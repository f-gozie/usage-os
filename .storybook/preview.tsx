import type { Preview } from "@storybook/react-vite";

// Same locally-bundled fonts + token contract the app uses, so stories render true.
import "@fontsource/anton/400.css";
import "@fontsource/jost/400.css";
import "@fontsource/jost/500.css";
import "@fontsource/jost/600.css";
import "@fontsource/jost/700.css";
import "../src/styles/tokens.css";
import "../src/index.css";

const preview: Preview = {
  globalTypes: {
    theme: {
      description: "Bauhaus theme",
      toolbar: {
        title: "Theme",
        icon: "paintbrush",
        items: [
          { value: "paper", title: "Paper" },
          { value: "warm", title: "Warm" },
          { value: "black", title: "Black" },
        ],
        dynamicTitle: true,
      },
    },
  },
  initialGlobals: { theme: "paper" },
  decorators: [
    (Story, context) => {
      const theme = (context.globals.theme as string) ?? "paper";
      return (
        <div data-theme={theme} style={{ background: "var(--bg)", color: "var(--fg)", padding: 32 }}>
          <Story />
        </div>
      );
    },
  ],
  parameters: {
    layout: "fullscreen",
  },
};

export default preview;
