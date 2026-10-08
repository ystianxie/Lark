import React, {useEffect, useLayoutEffect, useRef, useState} from 'react';

export default function ScreenshotOverlay({capture, result, onCancel, onSelected, onRetry, onReady}) {
    const imageRef = useRef(null);
    const overlayRef = useRef(null);
    const cancelingRef = useRef(false);
    const onCancelRef = useRef(onCancel);
    const [drag, setDrag] = useState(null);
    const [pointer, setPointer] = useState(null);

    useEffect(() => {
        onCancelRef.current = onCancel;
    }, [onCancel]);

    useLayoutEffect(() => {
        if (!capture || result) return;
        // The search input may still own focus while the screenshot window is
        // being shown. Listen on document so Escape works regardless of which
        // element currently has focus, and cancel before the host page handler.
        const onKeyDownCapture = event => {
            if (event.key !== 'Escape' || cancelingRef.current) return;
            event.preventDefault();
            event.stopPropagation();
            cancelingRef.current = true;
            onCancelRef.current?.();
        };
        const focusOverlay = () => overlayRef.current?.focus({preventScroll: true});
        document.addEventListener('keydown', onKeyDownCapture, true);
        // Native window focus can arrive just after React commits the overlay.
        // Refocus when the WebView receives that focus as well.
        window.addEventListener('focus', focusOverlay);
        // The overlay is mounted only after the native window has been shown
        // and focused, so focus it synchronously before the first paint.
        focusOverlay();
        // Reassert native window/WebView focus after this DOM node exists. On
        // Windows, setFocus can complete before React commits the overlay.
        Promise.resolve(onReady?.()).then(focusOverlay).catch(() => {});
        return () => {
            document.removeEventListener('keydown', onKeyDownCapture, true);
            window.removeEventListener('focus', focusOverlay);
        };
    }, [capture, result, onReady]);

    const point = event => ({x: event.clientX, y: event.clientY});
    const begin = event => {
        event.preventDefault();
        event.currentTarget.setPointerCapture?.(event.pointerId);
        const start = point(event);
        setDrag({start, current: start});
    };
    const move = event => {
        const current = point(event);
        setPointer(current);
        if (!drag) return;
        setDrag({...drag, current});
    };
    const finish = event => {
        if (!drag) return;
        event.preventDefault();
        const rect = imageRef.current?.getBoundingClientRect();
        const image = imageRef.current;
        const current = point(event);
        const left = Math.max(0, Math.min(drag.start.x, current.x) - (rect?.left || 0));
        const top = Math.max(0, Math.min(drag.start.y, current.y) - (rect?.top || 0));
        const right = Math.min(rect?.width || 0, Math.max(drag.start.x, current.x) - (rect?.left || 0));
        const bottom = Math.min(rect?.height || 0, Math.max(drag.start.y, current.y) - (rect?.top || 0));
        setDrag(null);
        if (!rect || !image || right - left < 4 || bottom - top < 4) return;

        const scaleX = image.naturalWidth / rect.width;
        const scaleY = image.naturalHeight / rect.height;
        const canvas = document.createElement('canvas');
        canvas.width = Math.max(1, Math.round((right - left) * scaleX));
        canvas.height = Math.max(1, Math.round((bottom - top) * scaleY));
        const context = canvas.getContext('2d');
        context.drawImage(
            image,
            Math.round(left * scaleX), Math.round(top * scaleY),
            Math.round((right - left) * scaleX), Math.round((bottom - top) * scaleY),
            0, 0, canvas.width, canvas.height,
        );
        onSelected(canvas.toDataURL('image/png'));
    };

    const selection = drag && {
        left: Math.min(drag.start.x, drag.current.x),
        top: Math.min(drag.start.y, drag.current.y),
        width: Math.abs(drag.current.x - drag.start.x),
        height: Math.abs(drag.current.y - drag.start.y),
    };

    if (result) {
        return <div style={{width: '100%', height: '100%', boxSizing: 'border-box', overflowY: 'auto', padding: '8px 0 12px', background: '#f7f7f5', color: '#252525'}}>
                <section style={{margin: '4px 15px 16px', padding: 14, border: '1px solid #e8e8e8', borderRadius: 10, background: '#fff', boxShadow: '0 1px 2px rgba(0,0,0,.03)'}}>
                    <div style={{display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12, marginBottom: 12}}>
                        <div style={{display: 'flex', alignItems: 'center', gap: 10, fontSize: 15, fontWeight: 600}}><span style={{fontSize: 20}}>✦</span>截图翻译</div>
                        <button type="button" onClick={onCancel} aria-label="关闭" style={{border: 0, background: 'transparent', fontSize: 22, lineHeight: 1, color: '#68707a', cursor: 'pointer', padding: '4px 8px', borderRadius: 6}}>×</button>
                    </div>
                    <div style={{display: 'flex', alignItems: 'center', gap: 12, padding: 12, border: '1px solid #e8eaf0', borderRadius: 8, background: '#fafbfc'}}>
                    <img src={result.dataUrl} alt="已选截图" style={{width: 96, height: 64, objectFit: 'cover', borderRadius: 6, border: '1px solid #e1e1e4'}}/>
                    <div style={{minWidth: 0, flex: 1}}>
                        <div style={{fontSize: 12, color: '#888', marginBottom: 3}}>截图翻译</div>
                        <div style={{fontSize: 13, color: result.error ? '#d93025' : '#555', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis'}}>{result.error || '截图已保存，等待 OCR 识别'}</div>
                    </div>
                    </div>
                    <div style={{display: 'flex', justifyContent: 'flex-end', marginTop: 12}}>
                        <button type="button" onClick={onRetry} style={{border: 0, borderRadius: 7, background: '#1677ff', color: '#fff', padding: '7px 13px', cursor: 'pointer', fontSize: 12}}>重新截图</button>
                    </div>
                </section>
        </div>;
    }

    if (!capture) return null;

    return <div ref={overlayRef} tabIndex={-1} role="application"
                 style={{position: 'fixed', inset: 0, zIndex: 99999, background: 'rgba(0,0,0,.32)', cursor: 'none', userSelect: 'none', outline: 'none'}}
                 onPointerDown={event => {
                     event.currentTarget.focus({preventScroll: true});
                     begin(event);
                 }} onPointerMove={move} onPointerUp={finish}>
        <img ref={imageRef} src={capture.dataUrl} alt="截图背景" draggable={false}
             style={{position: 'absolute', inset: 0, width: '100%', height: '100%', objectFit: 'fill', pointerEvents: 'none'}}/>
        <div style={{position: 'absolute', inset: 0, background: 'rgba(0,0,0,.32)', pointerEvents: 'none'}}/>
        {pointer && <div aria-hidden="true" style={{position: 'fixed', left: pointer.x, top: pointer.y, width: 24, height: 24, transform: 'translate(-50%, -50%)', pointerEvents: 'none', zIndex: 1, filter: 'drop-shadow(0 1px 2px rgba(0,0,0,.9))'}}>
            <span style={{position: 'absolute', left: 11, top: 0, width: 2, height: 24, borderRadius: 1, background: '#ffcf33', boxShadow: '0 0 0 1px #171717'}}/><span style={{position: 'absolute', left: 0, top: 11, width: 24, height: 2, borderRadius: 1, background: '#ffcf33', boxShadow: '0 0 0 1px #171717'}}/><span style={{position: 'absolute', left: 8, top: 8, width: 8, height: 8, borderRadius: '50%', background: '#ff4d6d', border: '1px solid white', boxSizing: 'border-box'}}/>
        </div>}
        {selection && <div style={{position: 'absolute', left: selection.left, top: selection.top, width: selection.width, height: selection.height, border: '2px solid #1677ff', background: 'rgba(22,119,255,.12)', boxSizing: 'border-box', pointerEvents: 'none'}}/>}
        <div style={{position: 'absolute', left: 18, top: 18, padding: '7px 11px', color: '#fff', background: 'rgba(0,0,0,.62)', borderRadius: 7, fontSize: 13, pointerEvents: 'none'}}>
            拖动选择截图区域 · Esc 取消
        </div>
    </div>;
}
