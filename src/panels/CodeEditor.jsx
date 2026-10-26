import {useEffect, useRef} from "react";
import {EditorState} from "@codemirror/state";
import {EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection} from "@codemirror/view";
import {defaultKeymap, history, historyKeymap, indentWithTab} from "@codemirror/commands";
import {javascript} from "@codemirror/lang-javascript";
import {python} from "@codemirror/lang-python";
import {oneDark} from "@codemirror/theme-one-dark";
import {indentUnit} from "@codemirror/language";

export default function CodeEditor({value, onChange, language = "javascript", autoFocus = false, className = ""}) {
    const hostRef = useRef(null);
    const viewRef = useRef(null);
    const onChangeRef = useRef(onChange);
    onChangeRef.current = onChange;

    useEffect(() => {
        if (!hostRef.current) return undefined;
        const languageExtension = language === "python" ? python() : javascript();
        const state = EditorState.create({
            doc: value || "",
            extensions: [
                lineNumbers(), history(), drawSelection(), highlightActiveLine(), EditorView.lineWrapping,
                keymap.of([indentWithTab, ...defaultKeymap, ...historyKeymap]), indentUnit.of("    "), languageExtension, oneDark,
                EditorView.updateListener.of(update => {
                    if (update.docChanged) onChangeRef.current(update.state.doc.toString());
                }),
                EditorView.theme({
                    "&": {height: "100%"}, ".cm-scroller": {overflow: "auto"},
                    ".cm-content": {fontFamily: "Consolas, monospace", fontSize: "13px", minHeight: "100%"},
                }),
            ],
        });
        const view = new EditorView({state, parent: hostRef.current});
        viewRef.current = view;
        if (autoFocus) view.focus();
        return () => { view.destroy(); viewRef.current = null; };
    }, [language]);

    useEffect(() => {
        const view = viewRef.current;
        if (!view || value === view.state.doc.toString()) return;
        view.dispatch({changes: {from: 0, to: view.state.doc.length, insert: value || ""}});
    }, [value]);

    return <div ref={hostRef} className={`code-editor ${className}`}/>;
}
