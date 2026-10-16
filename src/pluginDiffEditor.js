import * as monaco from "monaco-editor/editor/editor.api";
import "monaco-editor/editor/contrib/clipboard/browser/clipboard";
import "monaco-editor/editor/contrib/find/browser/findController";
import "monaco-editor/languages/definitions/javascript/register";
import "monaco-editor/languages/definitions/typescript/register";
import "monaco-editor/languages/definitions/python/register";
import "monaco-editor/languages/definitions/sql/register";
import "monaco-editor/languages/definitions/yaml/register";
import "monaco-editor/languages/features/json/register";
import EditorWorker from "monaco-editor/editor/editor.worker?worker";
import JsonWorker from "monaco-editor/language/json/json.worker?worker";

// Workers are bundled local assets; installed builds do not need a CDN.
self.MonacoEnvironment = {
  ...self.MonacoEnvironment,
  getWorker: (_moduleId, label) => label === "json" ? new JsonWorker() : new EditorWorker(),
};

export function mountPluginDiffEditor(container, options = {}) {
  const original = monaco.editor.createModel(options.original || "", options.language || "plaintext");
  const modified = monaco.editor.createModel(options.modified || "", options.language || "plaintext");
  let editor;
  try {
    editor = monaco.editor.createDiffEditor(container, {
      automaticLayout: true,
      originalEditable: true,
      readOnly: false,
      minimap: {enabled: false},
      scrollBeyondLastLine: false,
      fontSize: 13,
      renderSideBySide: options.sideBySide !== false,
      renderSideBySideInlineBreakpoint: 0,
      ignoreTrimWhitespace: !!options.ignoreTrimWhitespace,
      maxComputationTime: 2000,
      renderOverviewRuler: false,
    });
    editor.setModel({original, modified});
  } catch (error) {
    editor?.dispose(); original.dispose(); modified.dispose();
    throw error;
  }
  const updateTheme = () => monaco.editor.setTheme(document.documentElement.dataset.theme === "dark" ? "vs-dark" : "vs");
  updateTheme();
  const themeObserver = new MutationObserver(updateTheme);
  themeObserver.observe(document.documentElement, {attributes: true, attributeFilter: ["data-theme"]});
  const subscription = editor.onDidUpdateDiff(() => {
    const result = editor.getDiffComputationResult();
    options.onDiff?.({identical: result?.identical, quitEarly: result?.quitEarly, changes: result?.changes?.length || 0});
  });
  let disposed = false;
  return {
    getTexts: () => ({original: original.getValue(), modified: modified.getValue()}),
    setTexts: (left, right) => { original.setValue(left); modified.setValue(right); },
    setLanguage: language => { monaco.editor.setModelLanguage(original, language); monaco.editor.setModelLanguage(modified, language); },
    updateOptions: value => editor.updateOptions(value),
    navigate: forward => forward ? editor.goToDiff("next") : editor.goToDiff("previous"),
    dispose() {
      if (disposed) return;
      disposed = true;
      subscription.dispose(); themeObserver.disconnect(); editor.dispose(); original.dispose(); modified.dispose();
    },
  };
}

