import type { ComponentType, CSSProperties } from "react";

export type ControlType = "Button" | "Input" | "Password" | "TextArea" | "Select" | "Checkbox" | "Tabs";
export type ControlHandle = (() => void) & { update(props: Record<string, unknown>): void };
export interface PluginContext {
  theme?: Record<string, unknown>;
  input: { text: string; file?: unknown; selection?: unknown };
  ui: {
    mountControl(container: HTMLElement, type: ControlType, props?: Record<string, unknown>): ControlHandle;
    close?(): void;
    notify?: (...args: unknown[]) => unknown;
    [key: string]: unknown;
  };
  api: {
    getConfig(): Promise<Record<string, unknown>>;
    setConfig(values: Record<string, unknown>): Promise<unknown>;
    runPython?: (task: string, args?: Record<string, unknown>) => Promise<unknown>;
    openUrl?: (url: string) => Promise<unknown>;
    openFile?: (path: string) => Promise<unknown>;
    readClipboard?: () => Promise<unknown>;
    writeClipboard?: (text: string) => Promise<unknown>;
  };
}
export interface PluginRuntime {
  mount(container: HTMLElement, options?: { input?: string }): void;
  unmount(): void;
}
export function usePluginContext(): PluginContext;
export function definePlugin(App: ComponentType): (context: PluginContext) => PluginRuntime;
interface CommonProps {
  style?: CSSProperties;
  className?: string;
  title?: string;
  "aria-label"?: string;
  [key: string]: unknown;
}
export interface ButtonProps extends CommonProps {
  children?: string | number;
  label?: string | number;
  type?: "default" | "primary" | "dashed" | "link" | "text";
  icon?: "copy" | "delete" | "load" | "plus" | "save" | "send" | "upload" | "code" | "close";
  disabled?: boolean;
  loading?: boolean;
  onClick?: () => void;
}
export interface InputProps extends CommonProps {
  value?: string;
  placeholder?: string;
  disabled?: boolean;
  onChange?: (value: string) => void;
}
export interface CheckboxProps extends CommonProps {
  children?: string | number;
  checked?: boolean;
  disabled?: boolean;
  onChange?: (checked: boolean) => void;
}
export interface SelectProps extends CommonProps {
  value?: string | number | (string | number)[];
  options?: { value: string | number; label: string; disabled?: boolean }[];
  onChange?: (value: any, option: any) => void;
}
export interface TabsProps extends CommonProps {
  activeKey?: string;
  items: { key: string; label: string; disabled?: boolean }[];
  onChange?: (key: string) => void;
}
export const Button: ComponentType<ButtonProps>;
export const Input: ComponentType<InputProps>;
export const Password: ComponentType<InputProps>;
export const TextArea: ComponentType<InputProps & { rows?: number }>;
export const Checkbox: ComponentType<CheckboxProps>;
export const Select: ComponentType<SelectProps>;
export const Tabs: ComponentType<TabsProps>;
