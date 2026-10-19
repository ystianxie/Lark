import type { ComponentType } from "react";
import type { PluginContext, PluginRuntime } from "./index";
export type DevOverrides = Omit<Partial<PluginContext>, "input" | "ui" | "api"> & {
  input?: Partial<PluginContext["input"]>;
  ui?: Partial<PluginContext["ui"]>;
  api?: Partial<PluginContext["api"]>;
};
export function createDevContext(overrides?: DevOverrides): PluginContext;
export function previewPlugin(App: ComponentType, container: HTMLElement, context?: PluginContext): PluginRuntime;
