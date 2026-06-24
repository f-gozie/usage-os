import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(ts|tsx)"],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
  // No telemetry — consistent with the product's no-cloud/no-telemetry promise.
  core: {
    disableTelemetry: true,
  },
};

export default config;
